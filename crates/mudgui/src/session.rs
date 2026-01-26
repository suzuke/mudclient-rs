//! Session 管理模組
//!
//! 每個 Session 代表一個獨立的 MUD 連線，擁有：
//! - 獨立的 Telnet 連線
//! - 獨立的觸發器/別名（從 Profile 載入）
//! - 獨立的訊息緩衝區與日誌
//!
//! SessionManager 管理所有活躍的 Session，並提供分頁切換功能。

use std::collections::HashMap;
use std::time::Instant;
use mudcore::{
    Alias, AliasManager, Logger, ScriptEngine, Trigger, TriggerAction,
    TriggerManager, TriggerPattern, WindowManager, WindowMessage,
    MudContext, Path, PathManager, PathRecorder, LoopStatus,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc;
use crate::config::{AliasConfig, Profile, TriggerConfig};

// ============================================================================
// SessionId
// ============================================================================

/// Session 唯一識別碼
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

#[allow(dead_code)]
impl SessionId {
    /// 產生新的 SessionId
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// 取得內部 ID 值
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ConnectionStatus
// ============================================================================

/// 連線狀態
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected(String), // 包含伺服器資訊
    Reconnecting,      // 正在等待重連
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

// ============================================================================
// Command
// ============================================================================

/// 發送給網路執行緒的命令
#[derive(Debug)]
pub enum Command {
    Connect(String, u16, Option<String>, Option<String>), // Host, Port, Username, Password
    Send(String),
    Disconnect,
}

// ============================================================================
// ActiveTimer
// ============================================================================

/// 活躍的計時器
#[derive(Debug)]
pub struct ActiveTimer {
    /// 到期時間
    pub expires_at: Instant,
    /// 腳本代碼
    pub lua_code: String,
}

// ============================================================================
// Session
// ============================================================================

/// 單一連線會話
///
/// 包含一個 MUD 連線所需的所有狀態
pub struct Session {
    /// 唯一識別碼
    pub id: SessionId,
    
    /// 關聯的 Profile 名稱
    pub profile_name: String,
    
    /// 顯示名稱（用於分頁標題）
    pub display_name: String,

    // === 連線資訊 ===
    /// 主機位址
    pub host: String,
    /// 連接埠
    pub port: String,
    
    // === 帳號資訊 ===
    /// 登入帳號
    pub username: Option<String>,
    /// 登入密碼
    pub password: Option<String>,

    
    /// 連線狀態
    pub status: ConnectionStatus,
    
    /// 發送訊息到網路執行緒的 channel
    pub command_tx: Option<mpsc::Sender<Command>>,
    
    /// 從網路執行緒接收訊息的 channel
    pub message_rx: Option<mpsc::Receiver<String>>,
    
    /// 連線開始時間
    pub connected_at: Option<Instant>,

    // === 獨立的管理器（Profile 專屬） ===
    /// 別名管理器
    pub alias_manager: AliasManager,
    
    /// 觸發器管理器
    pub trigger_manager: TriggerManager,
    
    /// 路徑管理器
    pub path_manager: PathManager,
    
    /// 路徑記錄器
    pub path_recorder: PathRecorder,
    
    /// 腳本引擎
    pub script_engine: ScriptEngine,
    
    /// 視窗管理器
    pub window_manager: WindowManager,
    
    /// 日誌記錄器
    pub logger: Logger,

    // === 會話狀態 ===
    /// 輸入框內容
    pub input: String,
    
    /// 輸入歷史
    pub input_history: Vec<String>,
    
    /// 歷史索引
    pub history_index: Option<usize>,
    
    /// Tab 補齊前綴
    pub tab_completion_prefix: Option<String>,
    
    /// Tab 補齊索引
    pub tab_completion_index: usize,
    
    /// 是否發生了 Tab 補齊
    pub tab_completed: bool,

    /// Tab 補齊：上次補齊後的內容 (用於偵測手動修改)
    pub last_completed_input: Option<String>,
    
    /// 畫面單字字典（用於智慧補齊）
    pub screen_words: HashMap<String, WordMetadata>,
    
    /// 是否正在接收房間敘述
    #[allow(dead_code)]
    pub in_room_description: bool,
    
    /// 是否自動滾動到底部
    pub auto_scroll: bool,
    
    /// 是否需要在下一幀捲到底部
    pub scroll_to_bottom_on_next_frame: bool,

    // === 自動重連 ===
    /// 是否啟用自動重連
    pub auto_reconnect: bool,
    
    /// 重連等待時間點
    pub reconnect_delay_until: Option<Instant>,

    /// 最後活動時間
    #[allow(dead_code)]
    pub last_active: Instant,

    /// 活躍的計時器
    pub active_timers: Vec<ActiveTimer>,

    // === 多視窗預留 ===
    /// 當 Session 被拆分為獨立視窗時的視窗 ID
    #[allow(dead_code)]
    pub detached_window_id: Option<u64>,

    // === 防呆機制 ===
    /// 上一次發送的指令
    pub last_sent_command: Option<String>,
    
    /// 重複指令計數
    pub repeat_command_count: usize,
    
    /// 用於識別房間特徵的行緩衝區
    pub line_buffer: std::collections::VecDeque<String>,
}

/// 畫面單字的中繼資料
#[derive(Debug, Clone)]
pub struct WordMetadata {
    /// 最後一次出現的時間
    pub last_seen: Instant,
    /// 是否為 Mob/NPC 名稱
    pub is_mob: bool,
}

impl Session {
    /// 從 Profile 建立新的 Session
    pub fn from_profile(profile: &Profile) -> Self {
        let mut alias_manager = AliasManager::new();
        let mut trigger_manager = TriggerManager::new();
        let mut path_manager = PathManager::new();
        
        let username = profile.username.clone();
        let password = profile.password.clone();

        // 載入 Profile 的路徑
        for path_cfg in &profile.paths {
            let mut path = Path::new(&path_cfg.name, &path_cfg.value);
            path.category = path_cfg.category.clone();
            path_manager.add(path);
        }

        // 載入 Profile 的別名
        for alias_cfg in &profile.aliases {
            let mut alias = Alias::new(&alias_cfg.name, &alias_cfg.pattern, &alias_cfg.replacement);
            alias.category = alias_cfg.category.clone();
            alias.enabled = alias_cfg.enabled;
            alias.is_script = alias_cfg.is_script;
            alias_manager.add(alias);
        }

        // 載入 Profile 的觸發器
        for trigger_cfg in &profile.triggers {
            if let Some(trigger) = Self::create_trigger_from_config(trigger_cfg) {
                trigger_manager.add(trigger);
            }
        }

        // 建立日誌記錄器
        let mut logger = Logger::new();
        let log_path = format!(
            "logs/{}_{}.txt",
            profile.name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        let _ = logger.start(&log_path);

        Self {
            id: SessionId::new(),
            profile_name: profile.name.clone(),
            display_name: profile.display_name.clone(),
            host: profile.connection.host.clone(),
            port: profile.connection.port.clone(),
            username,
            password,
            status: ConnectionStatus::Disconnected,
            command_tx: None,
            message_rx: None,
            connected_at: None,
            alias_manager,
            trigger_manager,
            path_manager,
            path_recorder: PathRecorder::new(),
            script_engine: ScriptEngine::new(),
            window_manager: WindowManager::new(),
            logger,
            input: String::new(),
            input_history: Vec::new(),
            history_index: None,
            tab_completion_prefix: None,
            tab_completion_index: 0,
            tab_completed: false,
            last_completed_input: None,
            screen_words: HashMap::new(),
            in_room_description: false,
            auto_scroll: true,
            scroll_to_bottom_on_next_frame: false,
            auto_reconnect: true,
            reconnect_delay_until: None,
            last_active: Instant::now(),
            active_timers: Vec::new(),
            detached_window_id: None,
            last_sent_command: None,
            repeat_command_count: 0,
            line_buffer: std::collections::VecDeque::with_capacity(5),
        }
    }

    /// 從設定建立觸發器
    fn create_trigger_from_config(config: &TriggerConfig) -> Option<Trigger> {
        let clean_pattern = clean_pattern_string(&config.pattern);

        // 自動偵測正則表達式模式
        let pattern = if clean_pattern.contains("(.+)")
            || clean_pattern.contains("(.*)")
            || clean_pattern.contains("\\d")
            || clean_pattern.contains("[")
            || clean_pattern.contains("$")
            || clean_pattern.contains("^")
            || clean_pattern.contains("|")
            || clean_pattern.contains("?")
        {
            TriggerPattern::Regex(clean_pattern)
        } else {
            TriggerPattern::Contains(clean_pattern)
        };

        let mut trigger = Trigger::new(&config.name, pattern);

        if !config.action.is_empty() {
            if config.is_script {
                trigger = trigger.add_action(TriggerAction::ExecuteScript(config.action.clone()));
            } else {
                trigger = trigger.add_action(TriggerAction::SendCommand(config.action.clone()));
            }
        }

        trigger.category = config.category.clone();
        trigger.enabled = config.enabled;
        Some(trigger)
    }

    /// 合併全域觸發器/別名
    pub fn merge_global_config(
        &mut self,
        global_aliases: &[AliasConfig],
        global_triggers: &[TriggerConfig],
    ) {
        // 全域別名（加在 Profile 別名之前，優先度較低）
        for alias_cfg in global_aliases {
            // 如果 Profile 已有同名別名，跳過
            if self.alias_manager.get(&alias_cfg.name).is_some() {
                continue;
            }
            let mut alias = Alias::new(&alias_cfg.name, &alias_cfg.pattern, &alias_cfg.replacement);
            alias.category = alias_cfg.category.clone();
            alias.enabled = alias_cfg.enabled;
            alias.is_script = alias_cfg.is_script;
            self.alias_manager.add(alias);
        }

        // 全域觸發器
        for trigger_cfg in global_triggers {
            // 如果 Profile 已有同名觸發器，跳過
            if self.trigger_manager.get(&trigger_cfg.name).is_some() {
                continue;
            }
            if let Some(trigger) = Self::create_trigger_from_config(trigger_cfg) {
                self.trigger_manager.add(trigger);
            }
        }
    }


    /// 檢查並執行到期的計時器
    pub fn check_timers(&mut self) {
        if self.active_timers.is_empty() {
            return;
        }

        let now = Instant::now();
        let mut expired = Vec::new();

        self.active_timers.retain(|timer| {
            if now >= timer.expires_at {
                expired.push(timer.lua_code.clone());
                false
            } else {
                true
            }
        });

        for code in expired {
            if let Ok(context) = self.script_engine.execute_inline(&code, "TIMER_EXPIRED", &[], false) {
                self.apply_script_context(context);
            }
        }
    }

    /// 核心：將腳本執行結果套用到 Session
    pub fn apply_script_context(&mut self, context: MudContext) {
        // 1. 發送指令
        if let Some(tx) = &self.command_tx {
            for cmd in context.commands {
                let _ = tx.blocking_send(Command::Send(cmd));
            }
        }

        // 2. 本地回顯
        for echo in context.echos {
            self.handle_text(&echo, true);
        }

        // 3. 子視窗輸出
        for (win_id, text) in context.window_outputs {
            self.window_manager.route_message(
                &win_id,
                WindowMessage {
                    content: text,
                    preserve_ansi: true,
                },
            );
        }

        // 4. 計時器註冊
        let now = Instant::now();
        for (delay_ms, code) in context.timers {
            self.active_timers.push(ActiveTimer {
                expires_at: now + Duration::from_millis(delay_ms),
                lua_code: code,
            });
        }

        // 5. 日誌記錄
        for log_msg in context.log_messages {
            let _ = self.logger.log(&format!("[Script] {}", log_msg));
        }

        // 6. 觸發器狀態更新
        for (name, enabled) in context.trigger_updates {
            if let Some(trigger) = self.trigger_manager.get_mut(&name) {
                trigger.enabled = enabled;
                tracing::info!("Script updated trigger '{}' enabled: {}", name, enabled);
            }
        }
    }

    /// 處理接收到的文字與觸發器
    /// 處理接收到的文字與觸發器
    pub fn handle_text(&mut self, text: &str, is_echo: bool) -> bool {
        // 如果文字包含換行符，則逐行處理以確保狀態機能正確運作
        if text.contains('\n') {
            let mut result = true;
            for line in text.lines() {
                // 遞歸調用處理單行
                // 注意：這裡我們傳遞 is_echo 為 false，因為只有第一行可能是 echo（取決於調用上下文），
                // 但通常 handle_text 收到包含換行符的 msg 時都是來自伺服器的封包，非 echo。
                // 如果是使用者輸入的回顯，通常是單行。為求保險，若原為 echo 且是第一行才視為 echo?
                // 簡化起見：伺服器訊息通常是一大塊，is_echo=false。使用者輸入是單行，is_echo=true。
                // 所以這裡直接傳遞原始 is_echo flag 應該是可以的，因為這主要影響是否觸發 'look' 狀態。
                result &= self.handle_text(line, is_echo);
            }
            return result;
        }

        let mut gagged = false;
        let mut targets = vec!["main".to_string()];

        if !is_echo {
            // 處理觸發器
            let triggers = self.trigger_manager.process(text);
            
            // 暫存要執行的動作，避免借用衝突
            let mut pending_scripts = Vec::new();
            let mut pending_commands = Vec::new();
            
            // 執行觸發器動作
            for (trigger, m) in triggers {
                // Gag 檢查
                // if trigger.gag {
                //     gagged = true;
                // }

                // 執行動作
                for action in &trigger.actions {
                    match action {
                        TriggerAction::SendCommand(cmd) => {
                            if let Some(_tx) = &self.command_tx {
                                let mut expanded = cmd.clone();
                                for (i, cap) in m.captures.iter().enumerate() {
                                    expanded = expanded.replace(&format!("${}", i + 1), cap);
                                }
                                pending_commands.push(expanded);
                            }
                        }
                        TriggerAction::ExecuteScript(code) => {
                            pending_scripts.push((code.clone(), m.captures.clone()));
                        }
                        TriggerAction::RouteToWindow(win_id) => {
                            if !targets.contains(win_id) {
                                targets.push(win_id.clone());
                            }
                        }
                        TriggerAction::Gag => {
                            gagged = true;
                        }
                        _ => {}
                    }
                }
            }

            // 執行收集到的指令
            for cmd in pending_commands {
                // 使用 handle_user_input 處理觸發器指令，以支援分號拆分與別名
                self.handle_user_input(&cmd);
            }

            // 執行收集到的腳本
            for (code, captures) in pending_scripts {
                if let Ok(context) = self.script_engine.execute_inline(&code, text, &captures, false) {
                    self.apply_script_context(context);
                }
            }
        }

        // 如果被 Gag，則從主要輸出目標中移除 "main"
        if gagged {
            targets.retain(|t| t != "main");
        }

        // 路由到視窗
        for target_id in targets {
            self.window_manager.route_message(
                &target_id,
                WindowMessage {
                    content: text.to_string(),
                    preserve_ansi: !is_echo, 
                },
            );
        }

        // 提取單字用於自動補齊
        let clean_text = if text.contains('\x1b') {
            let re = regex::Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
            re.replace_all(text, "").to_string()
        } else {
            text.to_string()
        };

        let clean_lower = clean_text.to_lowercase();
        // 優化提示字元偵測：不分大小寫
        let is_prompt = clean_lower.contains('(') && clean_lower.contains('/') && 
                        (clean_lower.contains('h') || clean_lower.contains('m') || clean_lower.contains('v'));

        // 更新 Line Buffer (只存非空、非 Prompt、非系統訊息)
        if !text.trim().is_empty() && !is_prompt && !text.starts_with(">>>") {
            if self.line_buffer.len() >= 5 {
                self.line_buffer.pop_front();
            }
            self.line_buffer.push_back(text.trim().to_string());
        }

        let is_exit_line = clean_text.contains("[出口:");
        
        // --- 迴圈偵測 ---
        // --- 迴圈偵測 ---
        if is_exit_line && self.path_recorder.is_recording {
            // 嘗試從 buffer 抓取房間名稱 (通常是出口行的上一行)
            let room_name = if self.line_buffer.len() >= 2 {
                self.line_buffer.get(self.line_buffer.len() - 2).cloned().unwrap_or("Unknown".to_string())
            } else {
                "Unknown".to_string()
            };
            
            let signature = format!("{}|{}", room_name, clean_text.trim());
            
            let mut hasher = DefaultHasher::new();
            signature.hash(&mut hasher);
            let hash = hasher.finish();
            
            match self.path_recorder.record_room(hash) {
                LoopStatus::ExactLoop => {
                    self.system_message("⚠️ 偵測到迴圈！您回到了路徑起點或經過的原點 (Exact Loop)。");
                }
                LoopStatus::PotentialLoop => {
                    self.system_message("⚠️ 注意：此處場景與之前經過的地點極為相似 (Potential Loop)，但座標不同。");
                }
                LoopStatus::None => {}
            }
        }
        
        let trim_text = text.trim().to_lowercase();
        // 擴展方向指令偵測
        let is_dir = ["n", "s", "e", "w", "u", "d", "nw", "ne", "sw", "se", 
                      "north", "south", "east", "west", "up", "down", 
                      "northwest", "northeast", "southwest", "southeast"].contains(&trim_text.as_str());
        
        // 狀態機：進入房間描述模式
        if is_echo && (trim_text == "l" || trim_text == "look" || is_dir) {
            self.in_room_description = true;
        }

        // 狀態機：離開房間描述模式 (遇到 prompt)
        if is_prompt {
            self.in_room_description = false;
        }

        let is_exit_line = clean_text.contains("[出口:");
        let has_mob_brackets = clean_text.contains('(') && clean_text.contains(')');
        // 只要包含斜線且周圍有文字，很可能是 "中文名稱/English ID" 的格式
        let is_slash_line = clean_text.contains('/') && clean_text.len() > 5;

        // 如果符合任一條件，提取單字
        if has_mob_brackets || self.in_room_description || is_exit_line || is_slash_line {
            let now = Instant::now();
            
            // 1. 提取括號內的內容 (優先級高)
            let mob_re = regex::Regex::new(r"\(([^)]+)\)").unwrap();
            for cap in mob_re.captures_iter(&clean_text) {
                let content = &cap[1];
                for word in content.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                    if word.len() >= 2 && word.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                        self.screen_words.insert(word.to_string(), WordMetadata {
                            last_seen: now,
                            is_mob: true,
                        });
                    }
                }
            }

            // 2. 提取斜線後的內容 (針對 "中文/ID" 格式)
            if let Some(slash_idx) = clean_text.rfind('/') {
                let after_slash = &clean_text[slash_idx+1..];
                for word in after_slash.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                    if word.len() >= 2 && word.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                        self.screen_words.insert(word.to_string(), WordMetadata {
                            last_seen: now,
                            is_mob: true, // 假設斜線後通常是 ID
                        });
                    }
                }
            }

            // 3. 提取整行所有英文單字 (通用兜底)
            for word in clean_text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                if word.len() >= 2 && word.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    let entry = self.screen_words.entry(word.to_string()).or_insert(WordMetadata {
                        last_seen: now,
                        is_mob: false,
                    });
                    entry.last_seen = now;
                }
            }
        }

        // 限制字典大小
        if self.screen_words.len() > 1000 {
            let mut items: Vec<_> = self.screen_words.iter().map(|(k, m)| (k.clone(), m.last_seen)).collect();
            items.sort_by_key(|(_, t)| *t);
            // 移除最舊的 200 個
            for (k, _) in items.iter().take(200) {
                self.screen_words.remove(k);
            }
        }

        // 日誌記錄
        let _ = self.logger.log(text);

        self.last_active = Instant::now();

        true
    }

    /// 處理使用者輸入的指令 (包含特殊指令如 #loop, #delay, /lua)
    pub fn handle_user_input(&mut self, input: &str) {
        self.handle_user_input_with_depth(input, 0);
    }

    fn handle_user_input_with_depth(&mut self, input: &str, depth: usize) {
        let input = input.trim();
        
        // 防止無限遞迴
        if depth > 50 {
            self.system_message(&format!("Error: Command recursion limit reached for '{}'", input));
            return;
        }

        // 1. 分號拆分 (Semicolon Splitting)
        if input.contains(';') {
            for part in input.split(';') {
                self.handle_user_input_with_depth(part, depth + 1);
            }
            return;
        }

        // 2. 變數展開 (Variable Expansion)
        // 移至最前，確保 Trigger 和 Alias 都能看到展開後的變數
        let input = self.script_engine.expand_variables(input);

        // 3. 觸發器處理 (Local Echo Triggers)
        // 移至 Alias 之前，以支援 "Input Trigger" (針對玩家輸入的原始指令觸發)
        // 若 Alias 發生展開，遞迴呼叫會再次觸發針對展開後指令的 Trigger，達成多層觸發效果。
        tracing::info!("Checking input triggers for: '{}'", input);
        let matches = self.trigger_manager.process(&input);
        
        let mut pending_commands = Vec::new();
        let mut pending_scripts = Vec::new();

        for (trigger, m) in matches {
            tracing::info!("Match trigger: {}", trigger.name);
            for action in &trigger.actions {
                match action {
                    mudcore::TriggerAction::SendCommand(cmd) => {
                        let mut expanded = cmd.clone();
                        for (i, cap) in m.captures.iter().enumerate() {
                            expanded = expanded.replace(&format!("${}", i + 1), cap);
                        }
                        pending_commands.push(expanded);
                    }
                    mudcore::TriggerAction::ExecuteScript(code) => {
                        pending_scripts.push((code.clone(), m.captures.clone()));
                    }
                    _ => {}
                }
            }
        }
        
        for (script, captures) in pending_scripts {
            match self.script_engine.execute_inline(&script, &input, &captures, false) {
                Ok(ctx) => self.apply_script_context(ctx),
                Err(e) => {
                    tracing::error!("Trigger script error: {}", e);
                    self.system_message(&format!("Trigger Script Error: {}", e));
                }
            }
        }
        
        for cmd in pending_commands {
            self.handle_user_input_with_depth(&cmd, depth + 1);
        }

        // 4. Alias 處理
        use mudcore::alias::AliasMatchResult;
        match self.alias_manager.process_match(&input) {
            AliasMatchResult::Replacement(expanded) => {
                self.handle_user_input_with_depth(&expanded, depth + 1);
                return;
            }
            AliasMatchResult::Script(code) => {
                match self.script_engine.execute_inline(&code, &input, &[], false) {
                    Ok(ctx) => self.apply_script_context(ctx),
                    Err(e) => {
                        tracing::error!("Alias script error: {}", e);
                        self.system_message(&format!("Alias Script Error: {}", e));
                    }
                }
                return;
            }
            AliasMatchResult::None => {}
        }

        // 5. Path 與 Speedwalk 解析
        // 這邊處理兩件事：
        // a. Path Expansion: 如果輸入符合已定義的 path name，展開為 path value
        // b. Speedwalk Parsing: 如果輸入 (或展開後的內容) 對應 speedwalk 格式，則分解指令
        
        let path_value = if let Some(path) = self.path_manager.get(&input) {
            path.value.clone()
        } else {
            input.to_string()
        };

        // 嘗試解析為 Speedwalk
        // 需引入 mudcore::parse_speedwalk
        if let Some(commands) = mudcore::parse_speedwalk(&path_value) {
             for cmd in commands {
                 self.handle_user_input_with_depth(&cmd, depth + 1);
             }
             return;
        }

        // 如果發生了 path expansion 但不符合 speedwalk 格式 (例如純指令替換)，也需要遞迴處理
        if path_value != input {
             self.handle_user_input_with_depth(&path_value, depth + 1);
             return;
        }

        // 6. 處理特殊指令 (Client-Side Commands)
        if input.starts_with("#") || input.starts_with("/") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            let cmd = parts[0];

            match cmd {
                "#loop" => {
                    if parts.len() >= 3 {
                        if let Ok(count) = parts[1].parse::<usize>() {
                            let sub_cmd = parts[2..].join(" ");
                            for _ in 0..count {
                                self.handle_user_input_with_depth(&sub_cmd, depth + 1);
                            }
                            return;
                        }
                    }
                    self.system_message("Usage: #loop <count> <command>");
                    return;
                }
                "#delay" => {
                    if parts.len() >= 3 {
                        if let Ok(ms) = parts[1].parse::<u64>() {
                            let sub_cmd = parts[2..].join(" ");
                            let lua_code = format!("mud.send(\"{}\")", sub_cmd.replace("\"", "\\\""));
                            
                            self.active_timers.push(ActiveTimer {
                                expires_at: Instant::now() + std::time::Duration::from_millis(ms),
                                lua_code,
                            });
                            self.system_message(&format!("Delayed execution of '{}' by {}ms", sub_cmd, ms));
                            return;
                        }
                    }
                    self.system_message("Usage: #delay <ms> <command>");
                    return;
                }
                "/lua" => {
                    if parts.len() >= 2 {
                        let code = parts[1..].join(" ");
                        match self.script_engine.execute_inline(&code, "CLI", &[], true) {
                            Ok(ctx) => self.apply_script_context(ctx),
                            Err(e) => self.system_message(&format!("Lua Error: {}", e)),
                        }
                        return;
                    }
                    self.system_message("Usage: /lua <code>");
                    return;
                }
                "#var" => {
                    if parts.len() >= 3 {
                        let key = parts[1];
                        let value = parts[2..].join(" ");
                        let code = format!("mud.variables['{}'] = \"{}\"", key, value.replace("\"", "\\\""));
                        if let Err(e) = self.script_engine.execute_inline(&code, "CLI", &[], false) {
                            self.system_message(&format!("Failed to set variable: {}", e));
                        } else {
                            self.system_message(&format!("Variable '{}' set to '{}'", key, value));
                        }
                        return;
                    }
                    self.system_message("Usage: #var <key> <value>");
                    return;
                }
                "#unvar" => {
                    if parts.len() >= 2 {
                        let key = parts[1];
                        let code = format!("mud.variables['{}'] = nil", key);
                        if let Err(e) = self.script_engine.execute_inline(&code, "CLI", &[], false) {
                            self.system_message(&format!("Failed to unset variable: {}", e));
                        } else {
                            self.system_message(&format!("Variable '{}' unset", key));
                        }
                        return;
                    }
                    self.system_message("Usage: #unvar <key>");
                    return;
                }
                "#path" => {
                    if parts.len() < 2 {
                        self.system_message("Usage: #path <start|stop|loop|clear|undo|back|show|save>");
                        return;
                    }
                    match parts[1] {
                        "start" | "record" => {
                            self.path_recorder.start();
                            self.system_message("Path recording started.");
                        }
                        "stop" => {
                            self.path_recorder.stop();
                            self.system_message("Path recording stopped.");
                        }
                        "clear" => {
                            self.path_recorder.clear();
                            self.system_message("Path recording cleared.");
                        }
                        "simplify" | "optimize" => {
                            let old_len = self.path_recorder.recorded_commands.len();
                            self.path_recorder.simplify();
                            let new_len = self.path_recorder.recorded_commands.len();
                            self.system_message(&format!("Path simplified: {} -> {} steps (removed {} steps)", old_len, new_len, old_len - new_len));
                        }
                        "undo" => {
                            if let Some(removed) = self.path_recorder.pop_last() {
                                self.system_message(&format!("Undid last step: {}", removed));
                            } else {
                                self.system_message("No steps to undo.");
                            }
                        }
                        "back" => {
                            if self.path_recorder.is_recording {
                                self.system_message("Pausing recording for backtracking...");
                                self.path_recorder.stop();
                            }
                            
                            let reverse_path = self.path_recorder.get_reverse_path();
                            if reverse_path.is_empty() {
                                self.system_message("No path recorded to backtrack.");
                            } else {
                                self.system_message(&format!("Backtracking {} steps...", reverse_path.len()));
                                for cmd in reverse_path {
                                    self.handle_user_input_with_depth(&cmd, depth + 1);
                                }
                            }
                        }
                        "show" => {
                           let path_str = self.path_recorder.get_path_string();
                           if path_str.is_empty() {
                               self.system_message("Path is empty.");
                           } else {
                               self.system_message(&format!("Current Path: {}", path_str));
                           }
                        }
                        "save" => {
                            if parts.len() < 3 {
                                self.system_message("Usage: #path save <name>");
                            } else {
                                let name = parts[2];
                                let path_str = self.path_recorder.get_path_string();
                                if path_str.is_empty() {
                                    self.system_message("Cannot save empty path.");
                                } else {
                                    let path = mudcore::Path::new(name, &path_str);
                                    self.path_manager.add(path);
                                    self.system_message(&format!("Path saved as '{}'", name));
                                }
                            }
                        }
                        "loop" => {
                            if parts.len() < 3 {
                                self.system_message(&format!("Loop detection is currently: {}", if self.path_recorder.enable_loop_detection { "ON" } else { "OFF" }));
                                self.system_message("Usage: #path loop <on|off>");
                            } else {
                                match parts[2].to_lowercase().as_str() {
                                    "on" | "true" | "1" => {
                                        self.path_recorder.enable_loop_detection = true;
                                        self.system_message("Loop detection ENABLED.");
                                    }
                                    "off" | "false" | "0" => {
                                        self.path_recorder.enable_loop_detection = false;
                                        self.system_message("Loop detection DISABLED.");
                                    }
                                    _ => self.system_message("Usage: #path loop <on|off>"),
                                }
                            }
                        }
                        _ => {
                             self.system_message("Unknown path command. Usage: #path <start|stop|loop|clear|undo|back|show|save>");
                        }
                    }
                    return;
                }
                _ => {
                    // 如果不是已知指令，則視為普通內容發送
                }
            }
        }

        // 6. 標準指令處理 (本地回顯 + 發送)
        
        // 恢復回顯：
        self.window_manager.route_message("main", mudcore::window::WindowMessage {
            content: format!("{}{}\n", if input.is_empty() { "" } else { "\n" }, input), 
            preserve_ansi: true,
        });

        // Clone tx to avoid borrow check issues when calling system_message
        if let Some(tx) = self.command_tx.clone() {
            // === 防呆機制：檢查重複指令 ===
            let current_cmd = input.to_string();
            
            if let Some(last) = &self.last_sent_command {
                if last == &current_cmd {
                    self.repeat_command_count += 1;
                } else {
                    self.repeat_command_count = 1;
                    self.last_sent_command = Some(current_cmd.clone());
                }
            } else {
                self.repeat_command_count = 1;
                self.last_sent_command = Some(current_cmd.clone());
            }

            // 如果重複次數達到 20，自動插入 save
            if self.repeat_command_count >= 20 {
                self.system_message("Anti-spam: Repeated command limit reached (20). Auto-inserting 'save'.");
                let _ = tx.blocking_send(crate::session::Command::Send("save".to_string()));
                // 重置計數器，讓使用者可以繼續輸入（或根據需求重置為 1）
                self.repeat_command_count = 0;
            }
            
            // 記錄路徑 (在送出前記錄)
            if self.path_recorder.is_recording {
                 self.path_recorder.record(&input);
            }

            let _ = tx.blocking_send(crate::session::Command::Send(input.to_string()));
        }
    }

    /// 顯示系統訊息
    fn system_message(&mut self, msg: &str) {
        self.window_manager.route_message("main", mudcore::window::WindowMessage {
            content: format!("\n[System] {}\n", msg),
            preserve_ansi: true,
        });
    }




    /// 取得分頁標題
    pub fn tab_title(&self) -> String {
        let status_icon = match &self.status {
            ConnectionStatus::Disconnected => "⚪",
            ConnectionStatus::Connecting => "🔄",
            ConnectionStatus::Connected(_) => "🟢",
            ConnectionStatus::Reconnecting => "🟡",
        };
        format!("{} {}", status_icon, self.display_name)
    }

    /// 是否已連線
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected(_))
    }

    /// 是否正在連線
    #[allow(dead_code)]
    pub fn is_connecting(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connecting | ConnectionStatus::Reconnecting)
    }
}

// ============================================================================
// SessionManager
// ============================================================================

/// Session 管理器
///
/// 管理所有活躍的 Session 並提供分頁切換功能
pub struct SessionManager {
    /// 所有活躍的 Session（依序）
    sessions: Vec<Session>,
    
    /// 目前選中的分頁索引
    active_index: usize,
    
    /// 全域別名（套用到所有 Session）
    global_aliases: Vec<AliasConfig>,
    
    /// 全域觸發器（套用到所有 Session）
    global_triggers: Vec<TriggerConfig>,
}

#[allow(dead_code)]
impl SessionManager {
    /// 建立新的 SessionManager
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active_index: 0,
            global_aliases: Vec::new(),
            global_triggers: Vec::new(),
        }
    }

    /// 設定全域別名/觸發器
    pub fn set_global_config(
        &mut self,
        aliases: Vec<AliasConfig>,
        triggers: Vec<TriggerConfig>,
    ) {
        self.global_aliases = aliases;
        self.global_triggers = triggers;
    }

    /// 從 Profile 建立並新增 Session
    pub fn create_session(&mut self, profile: &Profile) -> SessionId {
        let mut session = Session::from_profile(profile);
        session.merge_global_config(&self.global_aliases, &self.global_triggers);
        
        let id = session.id;
        self.sessions.push(session);
        
        // 自動切換到新分頁
        self.active_index = self.sessions.len() - 1;
        
        id
    }

    /// 關閉 Session
    pub fn close_session(&mut self, id: SessionId) -> bool {
        if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
            self.sessions.remove(pos);
            
            // 調整 active_index
            if self.active_index >= self.sessions.len() && !self.sessions.is_empty() {
                self.active_index = self.sessions.len() - 1;
            }
            return true;
        }
        false
    }

    /// 取得目前選中的 Session
    pub fn active_session(&self) -> Option<&Session> {
        self.sessions.get(self.active_index)
    }

    /// 取得目前選中的 Session（可變）
    pub fn active_session_mut(&mut self) -> Option<&mut Session> {
        self.sessions.get_mut(self.active_index)
    }

    /// 取得目前選中的 Session ID
    pub fn active_id(&self) -> Option<SessionId> {
        self.active_session().map(|s| s.id)
    }

    /// 依 ID 取得 Session
    pub fn get(&self, id: SessionId) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// 依 ID 取得 Session（可變）
    pub fn get_mut(&mut self, id: SessionId) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    /// 切換到指定分頁
    pub fn switch_tab(&mut self, index: usize) -> bool {
        if index < self.sessions.len() {
            self.active_index = index;
            true
        } else {
            false
        }
    }

    /// 切換到上一個分頁
    pub fn prev_tab(&mut self) {
        if !self.sessions.is_empty() && self.active_index > 0 {
            self.active_index -= 1;
        }
    }

    /// 切換到下一個分頁
    pub fn next_tab(&mut self) {
        if self.active_index + 1 < self.sessions.len() {
            self.active_index += 1;
        }
    }

    /// 取得所有 Session 的參照（用於渲染分頁列）
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// 取得目前分頁索引
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Session 數量
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// 是否為空
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// 依索引取得 Session（可變）
    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut Session> {
        self.sessions.get_mut(index)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 工具函數
// ============================================================================

/// 清理可能的 Debug 格式
fn clean_pattern_string(pattern: &str) -> String {
    let s = pattern.trim();
    // 移除可能的 Contains(...) 或 Regex(...) 包裝
    if s.starts_with("Contains(\"") && s.ends_with("\")") {
        return s[10..s.len() - 2].to_string();
    }
    if s.starts_with("Regex(\"") && s.ends_with("\")") {
        return s[7..s.len() - 2].to_string();
    }
    s.to_string()
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConnectionConfig, Profile};

    #[test]
    fn test_session_id_unique() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_from_profile() {
        let profile = Profile {
            name: "test".to_string(),
            display_name: "測試".to_string(),
            connection: ConnectionConfig {
                host: "localhost".to_string(),
                port: "7777".to_string(),
            },
            aliases: vec![],
            triggers: vec![],
            script_paths: vec![],
            username: None,
            password: None,
            created_at: 0,
            last_connected: None,
        };

        let session = Session::from_profile(&profile);
        assert_eq!(session.profile_name, "test");
        assert_eq!(session.display_name, "測試");
        assert_eq!(session.host, "localhost");
    }

    #[test]
    fn test_session_manager_create_and_switch() {
        let mut manager = SessionManager::new();
        
        let profile1 = Profile::new("p1", "Profile 1")
            .with_connection("host1", "7777");
        let profile2 = Profile::new("p2", "Profile 2")
            .with_connection("host2", "7778");

        let id1 = manager.create_session(&profile1);
        let id2 = manager.create_session(&profile2);

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index(), 1); // 自動切到新分頁

        manager.switch_tab(0);
        assert_eq!(manager.active_session().unwrap().id, id1);

        manager.switch_tab(1);
        assert_eq!(manager.active_session().unwrap().id, id2);
    }
}
