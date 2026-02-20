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
    map::Room,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc;
use crate::api::SharedApiState;
use crate::config::{AliasConfig, Profile, TriggerConfig};
use lazy_static::lazy_static;

lazy_static! {
    static ref ANSI_STRIP_RE: regex::Regex = regex::Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
    static ref MOB_BRACKET_RE: regex::Regex = regex::Regex::new(r"\(([^)]+)\)").unwrap();
}

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
    /// 指令回應收集：網路執行緒先 drain 管線，再發送指令並收集回應
    CollectResponse { command: String, callback_code: String },
    Disconnect,
}

/// 網路執行緒傳回的訊息
#[derive(Debug)]
pub enum NetMessage {
    /// 一般文字資料
    Text(String, Vec<u8>),
    /// 指令回應收集完成
    CollectedResponse { lines: Vec<String>, callback_code: String },
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

    /// 用戶筆記
    pub notes: String,

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
    
    /// 從網路執行緒接收訊息的 channel (內容, 原始位元組寬度)
    pub message_rx: Option<mpsc::Receiver<NetMessage>>,
    
    /// 連線開始時間
    pub connected_at: Option<Instant>,

    /// 當前房間
    pub current_room: Option<Room>,

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
    
    /// [穩定化] 專用的伺服器純淨緩衝區，用於 Room ID 計算
    pub server_buffer: std::collections::VecDeque<String>,

    /// API 共享狀態（供 HTTP API 使用）
    pub api_state: SharedApiState,
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
    pub fn from_profile(profile: &Profile, api_state: SharedApiState) -> Self {
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

        let mut session = Self {
            id: SessionId::new(),
            profile_name: profile.name.clone(),
            display_name: profile.display_name.clone(),
            notes: profile.notes.clone(),
            host: profile.connection.host.clone(),
            port: profile.connection.port.clone(),
            username,
            password,
            status: ConnectionStatus::Disconnected,
            command_tx: None,
            message_rx: None,
            connected_at: None,
            current_room: None,
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
            line_buffer: std::collections::VecDeque::with_capacity(20),
            server_buffer: std::collections::VecDeque::with_capacity(20),
            api_state,
        };

        // 自動載入 scripts/ 目錄下的腳本
        session.load_startup_scripts();

        session
    }

    /// 從設定建立觸發器
    pub fn create_trigger_from_config(config: &TriggerConfig) -> Option<Trigger> {
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



    /// 嘗試從行緩衝區偵測房間資訊
    fn detect_room_info(&mut self, current_line: &str) {
        // 先移除 ANSI 顏色碼以便比對
        let clean_current = if current_line.contains('\x1b') {
            ANSI_STRIP_RE.replace_all(current_line, "").to_string()
        } else {
            current_line.to_string()
        };

        // 1. 檢查是否為出口行
        // 格式範例: [出口: 北 南] 或 [Exits: north south]
        if !clean_current.starts_with("[出口:") && !clean_current.starts_with("[Exits:") {
            return;
        }

        // 2. 解析出口
        let exits_str = if let Some(s) = clean_current.strip_prefix("[出口:") {
            s.trim_end_matches(']')
        } else if let Some(s) = clean_current.strip_prefix("[Exits:") {
            s.trim_end_matches(']')
        } else {
            return;
        };

        let exits: Vec<String> = exits_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // 3. 回溯尋找房間名稱與描述
        // 策略: 往回找上一個出口行，中間就是名稱與描述
        let n = self.server_buffer.len();
        if n < 2 { return; }
        
        // line_buffer 的最後一行 (n-1) 應該就是 current_line (因為剛 push 進去)
        // 我們從 n-2 開始往回找
        let mut name_index = 0;
        let mut found_prev_exit = false;
        
        // 限制回溯行數，避免讀到太久以前的雜訊
        let scan_limit = 12; // 假設房間描述不會超過 12 行
        let start_index = if n > scan_limit + 1 { n - 1 - scan_limit } else { 0 };

        for i in (start_index..n-1).rev() {
            let line = &self.server_buffer[i];
            // 同樣需要移除 ANSI
            let clean_line = if line.contains('\x1b') {
                ANSI_STRIP_RE.replace_all(line, "").to_string()
            } else {
                line.to_string()
            };
            
            // 檢查是否為上一個出口行
            if clean_line.starts_with("[出口:") || clean_line.starts_with("[Exits:") {
                name_index = i + 1;
                found_prev_exit = true;
                break;
            }

            // 檢查是否為 "停止訊號" (Stop Signals)
            // 這些特徵通常出現在房間名稱之前，表示我們已經回溯到了上一區塊的結尾
            let trim_line = clean_line.trim();
            
            // 1. 指令回顯 (例如 "> w")
            if trim_line.starts_with('>') {
                name_index = i + 1;
                found_prev_exit = true; // 視同找到分隔線
                break;
            }
            
            // 2. Prompt (如果沒被 filter 掉)
            if trim_line.starts_with('(') && trim_line.contains('/') && trim_line.contains(')') {
                name_index = i + 1;
                found_prev_exit = true;
                break;
            }
            
            // 3. Mob/物件 (通常包含 "/" 或是特定結尾)
            // 簡單啟發式：如果包含 "/" 且不是出口行，極有可能是 Mob id
            if trim_line.contains('/') {
                 name_index = i + 1;
                 found_prev_exit = true;
                 break;
            }
            
            // 4. 系統訊息
            if trim_line.starts_with("[System]") {
                name_index = i + 1;
                found_prev_exit = true;
                break;
            }
        }
        
        // 如果沒找到明確的分隔線，且我們回溯到了 start_index
        if !found_prev_exit {
            // 如果 buffer 夠大，我們假設 start_index 就是開始？
            // 這有風險，但比完全不更新好。
            if self.server_buffer.len() >= 20 && start_index > 0 {
                return;
            }
            name_index = 0;
        }
        
        if name_index >= n - 1 {
            // 沒有中間內容 (例如連續兩個出口行? 不太可能，除非快速移動且 gag 了描述)
            return;
        }
        
        // 取得名稱 (移除 ANSI)
        let raw_name = &self.server_buffer[name_index];
        let name = if raw_name.contains('\x1b') {
            ANSI_STRIP_RE.replace_all(raw_name, "").to_string()
        } else {
            raw_name.to_string()
        };
        
        // 描述是中間的部分 (移除 ANSI)
        // [修正] 排除提示字元行 (Prompt)，這些行通常動態變化，會破壞 Hash 穩定性
        let mut desc_lines = Vec::new();
        for i in (name_index + 1)..n-1 {
            let raw_line = &self.server_buffer[i];
            let clean_line = if raw_line.contains('\x1b') {
                ANSI_STRIP_RE.replace_all(raw_line, "").to_string()
            } else {
                raw_line.to_string()
            };
            
            // 偵測並排除干擾行，但必須保留標記為 [出口: 的行
            let trimmed = clean_line.trim();
            let is_prompt = (trimmed.starts_with('(') && clean_line.contains("hp")) || 
                            (trimmed.starts_with('[') && clean_line.contains("hp"));
            let is_exits = trimmed.starts_with("[出口:");
            let is_script = trimmed.starts_with('[') && !is_exits;
            
            let is_noise = is_prompt || is_script;
            
            if !is_noise {
                desc_lines.push(clean_line);
            } else {
                tracing::debug!("Excluding noise from room hash: {}", trimmed);
            }
        }
        let description = desc_lines.join("\n");
        
        // 建立 Room 物件
        let room = Room::new(&name, &description, exits.clone());
        let id = room.hash(true); // 預設使用 Strict 模式記錄 Log
        
        // 總是更新，確保 Look 能觸發
        // 只有 ID 改變時才寫入 Log，避免刷屏
        let id_changed = if let Some(old_room) = &self.current_room {
             old_room.hash(true) != id
        } else {
            true
        };

        if id_changed {
            tracing::info!("Room Detected: {} (ID: {})", name, id);
        }
        
        self.current_room = Some(room.clone());
        self.script_engine.set_current_room(Some(room));

        // 更新 API 共享狀態的房間資訊
        if let Ok(mut api) = self.api_state.lock() {
            api.current_room = Some(crate::api::RoomInfo {
                name: name.clone(),
                exits: exits.clone(),
                room_id: Some(id.clone()),
                description: description.clone(),
            });
        }
        
        // 觸發 Lua Hook: on_room_detected(id, name)
        // 這裡我們傳入預設的 strict ID，但 Lua 腳本可以自己呼叫 mud.get_current_room() 取得更多資訊
        // 或者 mud.get_current_room_id(false) 取得 lax ID
        // 觸發 Lua Hook: on_room_detected(id, name)
        match self.script_engine.invoke_hook("on_room_detected", &id, &name) {
            Ok(Some(context)) => {
                self.apply_script_context(context);
            }
            Ok(None) => {}, // Hook not defined
            Err(e) => {
                 tracing::error!("on_room_detected hook error: {}", e);
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
            match self.script_engine.execute_inline(&code, "TIMER_EXPIRED", &[], false) {
                Ok(context) => self.apply_script_context(context),
                Err(e) => {
                    tracing::error!("Timer execution failed: {}\nCode: {}", e, code);
                    self.system_message(&format!("Timer Error: {}", e));
                }
            }
        }
    }

    /// 載入並執行 scripts/ 目錄下的所有 .lua 腳本
    /// 搜尋順序：1) 工作目錄下的 scripts/  2) 執行檔旁的 scripts/
    /// 載入順序：Phase 1 = scripts/modules/*.lua（字母排序），Phase 2 = scripts/*.lua（字母排序）
    fn load_startup_scripts(&mut self) {
        let scripts_dir = Self::resolve_scripts_dir();

        let Some(scripts_dir) = scripts_dir else {
            let _ = self.logger.log("找不到 scripts 目錄（工作目錄與執行檔目錄皆無）");
            return;
        };

        // 設定 scripts_dir 的絕對路徑，供 dofile 查找
        self.script_engine.set_scripts_dir(scripts_dir.to_string_lossy().as_ref());

        // --- Phase 1: 已移除 ---
        // (交由 Lua 端的 require 來管理 modules/ 目錄的相依性，避免重複載入覆蓋狀態)

        // --- Phase 2: 載入頂層 scripts/*.lua ---
        match std::fs::read_dir(&scripts_dir) {
            Ok(entries) => {
                let mut valid_scripts: Vec<std::path::PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().map_or(false, |ext| ext == "lua"))
                    .collect();
                valid_scripts.sort();

                for path in valid_scripts {
                    if let Ok(code) = std::fs::read_to_string(&path) {
                        let filename = path.file_name().unwrap_or_default().to_string_lossy();
                        match self.script_engine.execute_inline(&code, "STARTUP", &[], false) {
                            Ok(context) => {
                                self.apply_script_context(context);
                                let _ = self.logger.log(&format!("已自動載入腳本: {}", filename));
                                self.window_manager.route_message("main", mudcore::WindowMessage {
                                    content: format!("\n[System] 自動載入腳本: {}\n", filename),
                                    preserve_ansi: true,
                                    byte_widths: Vec::new(),
                                    repeat_count: 1,
                                });
                            }
                            Err(e) => {
                                let msg = format!("腳本載入錯誤 ({}): {}", filename, e);
                                let _ = self.logger.log(&msg);
                                self.system_message(&msg);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = self.logger.log(&format!("無法讀取 scripts 目錄: {}", e));
            }
        }
    }

    /// 解析 scripts 目錄的實際位置
    /// 搜尋順序：1) 工作目錄 → 2) .app bundle Resources → 3) 執行檔旁邊
    fn resolve_scripts_dir() -> Option<std::path::PathBuf> {
        // 1. 工作目錄下的 scripts/
        let cwd_scripts = std::path::Path::new("scripts");
        if cwd_scripts.exists() && cwd_scripts.is_dir() {
            return std::fs::canonicalize(cwd_scripts).ok();
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                // 2. macOS .app bundle: Contents/MacOS/mudgui → Contents/Resources/scripts/
                if exe_dir.ends_with("Contents/MacOS") {
                    if let Some(contents_dir) = exe_dir.parent() {
                        let res_scripts = contents_dir.join("Resources").join("scripts");
                        if res_scripts.exists() && res_scripts.is_dir() {
                            return std::fs::canonicalize(res_scripts).ok();
                        }
                    }
                }

                // 3. 執行檔旁的 scripts/
                let exe_scripts = exe_dir.join("scripts");
                if exe_scripts.exists() && exe_scripts.is_dir() {
                    return std::fs::canonicalize(exe_scripts).ok();
                }
            }
        }

        None
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
                    byte_widths: Vec::new(),
                    repeat_count: 1,
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
        
        // 7. 日誌控制
        if let Some(control) = context.log_control {
            match control {
                mudcore::script::LogControl::Start(path) => {
                    if let Err(e) = self.logger.start(&path) {
                        let _ = self.logger.log(&format!("無法啟動日誌: {}", e));
                        self.system_message(&format!("⚠️ 無法啟動日誌 '{}': {}", path, e));
                    } else {
                        self.system_message(&format!("📝 開始記錄日誌至 '{}'", path));
                    }
                }
                mudcore::script::LogControl::Stop => {
                    if let Err(e) = self.logger.stop() {
                        let _ = self.logger.log(&format!("無法停止日誌: {}", e));
                    } else {
                        self.system_message("🛑 停止記錄日誌");
                    }
                }
            }
        }

        // 8. 指令回應收集器（透過 Command channel 發送到網路執行緒）
        for (cmd, callback_code) in context.response_collectors {
            if let Some(tx) = &self.command_tx {
                let _ = tx.blocking_send(Command::CollectResponse {
                    command: cmd.clone(),
                    callback_code,
                });
                tracing::info!("CollectResponse: queued for command '{}'", cmd);
            }
        }
    }

    /// 處理接收到的文字與觸發器
    pub fn handle_text(&mut self, text: &str, is_echo: bool) -> bool {
        self.handle_text_with_widths(text, is_echo, None)
    }

    /// 帶有位元組寬度的文字處理
    pub fn handle_text_with_widths(&mut self, text: &str, is_echo: bool, byte_widths: Option<&[u8]>) -> bool {
        // 如果文字包含換行符，則逐行處理
        if text.contains('\n') {
            let mut result = true;
            let mut current_pos = 0;
            let lines: Vec<&str> = text.split('\n').collect();
            for line in lines {
                // 計算該行的位元組寬度切片
                let line_widths = if let Some(widths) = byte_widths {
                    let char_count = line.chars().count();
                    let start = current_pos;
                    let end = (start + char_count).min(widths.len());
                    
                    // 下一行的起始位置需跳過這行的字元數 + 1 (換行符)
                    current_pos += char_count + 1;
                    
                    if start < widths.len() {
                        Some(&widths[start..end])
                    } else {
                        Some(&[] as &[u8])
                    }
                } else {
                    None
                };

                result &= self.handle_text_with_widths(line, is_echo, line_widths);
            }
            return result;
        }



        let mut gagged = false;
        let mut targets = vec!["main".to_string()];

        // 提前計算 clean_text 以供各處使用
        let clean_text = if text.contains('\x1b') {
            ANSI_STRIP_RE.replace_all(text, "").to_string()
        } else {
            text.to_string()
        };

        // [穩定化] 先更新 Server Buffer 與 Room ID，確保 Lua Hook 能取得最新 Room ID
        if !is_echo && !text.trim().is_empty() && !text.starts_with(">>>") {
            let clean_lower_pre = clean_text.to_lowercase();
            let is_prompt_pre = clean_lower_pre.starts_with('(') && clean_lower_pre.contains('/') && clean_lower_pre.contains(')');
            if !is_prompt_pre {
                if self.server_buffer.len() >= 20 {
                    self.server_buffer.pop_front();
                }
                self.server_buffer.push_back(text.trim().to_string());
                
                // 嘗試偵測房間 (改用純淨緩衝區)
                self.detect_room_info(text.trim());
            }
        }

        // 0. 呼叫全域鉤子 (Global Hook)
        // 這允許 Lua 腳本直接處理每一行伺服器訊息與回顯，無需透過正則表達式觸發器
        match self.script_engine.invoke_hook("on_server_message", text, &clean_text) {
            Ok(Some(context)) => {
                if context.gag {
                    gagged = true;
                }
                self.apply_script_context(context);
            },
            Ok(None) => {}, // Hook not defined
            Err(e) => {
                tracing::error!("on_server_message hook error: {}", e);
                self.window_manager.send_to_main(format!("{{r[System] Hook Error: {}{{x}}", e));
            }
        }

        if !is_echo {
            // 提取單字用於自動補齊與狀態判斷
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
                    if context.gag {
                        gagged = true;
                    }
                    self.apply_script_context(context);
                }
            }
        }

        // 如果被 Gag，則從主要輸出目標中移除 "main"
        if gagged {
            targets.retain(|t| t != "main");
        }

        // clean_text 已經在開頭計算過了

        let clean_lower = clean_text.to_lowercase();
        // 優化提示字元偵測：不分大小寫
        // 1. 標準 Prompt: (hp.../...)
        // 2. 純數值 Prompt: (123/123 456/456 ...)
        let is_prompt = clean_lower.starts_with('(') && clean_lower.contains('/') && clean_lower.contains(')');

        // 如果是房間敘述，且非出口行、非 Prompt、非 Echo，則進行標點轉換
        let is_exit_line = clean_text.contains("[出口:");
        
        let final_text = text.to_string();
        let mut final_widths = Vec::new();

        // 預查原始寬度
        if let Some(widths) = byte_widths {
            final_widths = widths.to_vec();
        } else {
            for ch in text.chars() {
                if ch.is_ascii() {
                    final_widths.push(1);
                } else {
                    final_widths.push(2);
                }
            }
        }

        // 判定是否為指令回顯，並提取核心內容用於狀態判斷
        let (is_command_echo, detection_text) = if is_echo && clean_text.trim().starts_with('>') {
            (true, clean_text.trim().trim_start_matches('>').trim())
        } else {
            (is_echo, clean_text.trim())
        };
        let trim_detection = detection_text.to_lowercase();
        let is_dir_cmd = ["n", "s", "e", "w", "u", "d", "nw", "ne", "sw", "se", 
                          "north", "south", "east", "west", "up", "down", 
                          "northwest", "northeast", "southwest", "southeast"].contains(&trim_detection.as_str());

        // 路由到視窗
        for target_id in targets {
            let msg = WindowMessage {
                content: final_text.clone(),
                preserve_ansi: !is_echo,
                byte_widths: final_widths.clone(),
                repeat_count: 1,
            };
            
            self.window_manager.route_message_with_widths(
                &target_id,
                msg,
            );
        }

        // 更新 Line Buffer (通用緩衝區，包含 Echo)
        if !text.trim().is_empty() && !is_prompt && !text.starts_with(">>>") {
            if self.line_buffer.len() >= 20 {
                self.line_buffer.pop_front();
            }
            self.line_buffer.push_back(text.trim().to_string());
        }


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
        
        // 狀態機：進入房間描述模式
        if is_command_echo && (trim_detection == "l" || trim_detection == "look" || is_dir_cmd) {
            self.in_room_description = true;
        }

        // 狀態機：離開房間描述模式 (遇到 prompt)
        if is_prompt {
            self.in_room_description = false;
        }
        let has_mob_brackets = clean_text.contains('(') && clean_text.contains(')');
        // 只要包含斜線且周圍有文字，很可能是 "中文名稱/English ID" 的格式
        let is_slash_line = clean_text.contains('/') && clean_text.len() > 5;

        // 如果符合任一條件，提取單字
        if has_mob_brackets || self.in_room_description || is_exit_line || is_slash_line {
            let now = Instant::now();
            
            // 1. 提取括號內的內容 (優先級高)
            for cap in MOB_BRACKET_RE.captures_iter(&clean_text) {
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
            let cutoff = Instant::now() - Duration::from_secs(300); // 5 分鐘前
            self.screen_words.retain(|_, m| m.last_seen > cutoff);
        }

        // 日誌記錄
        let _ = self.logger.log(text);

        // 推送純文字訊息到 API 共享狀態
        if !text.trim().is_empty() {
            if let Ok(mut api) = self.api_state.lock() {
                api.push_message(clean_text.replace('\r', "").trim().to_string());
            }
        }

        self.last_active = Instant::now();

        true
    }

    /// 處理網路執行緒收集的指令回應
    pub fn execute_collected_response(&mut self, lines: Vec<String>, callback_code: String) {
        // 構建 Lua table 字串
        let mut lines_lua = String::from("{");
        for (i, line) in lines.iter().enumerate() {
            if i > 0 { lines_lua.push(','); }
            let escaped = line.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
            lines_lua.push('"');
            lines_lua.push_str(&escaped);
            lines_lua.push('"');
        }
        lines_lua.push('}');
        
        let code = format!("_G._collected_lines = {} \n {}", lines_lua, callback_code);
        tracing::info!("CollectedResponse: {} lines, executing callback", lines.len());
        
        if let Ok(ctx) = self.script_engine.execute_inline(&code, "COLLECT_RESPONSE", &[], false) {
            self.apply_script_context(ctx);
        }
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

        // 標準指令處理 (本地回顯 + 發送)
        // 改進回顯格式：緊隨 Prompt 且使用明顯前綴，並透過 handle_text 觸發狀態機
        self.handle_text(&format!("> {}\n", input), true);

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

            // 觸發 Lua Hook: on_command(command)
            if let Ok(Some(context)) = self.script_engine.invoke_hook("on_command", &input, &input) {
                 self.apply_script_context(context);
            }

            let _ = tx.blocking_send(crate::session::Command::Send(input.to_string()));
            tracing::info!("[DEBUG] Command sent to channel: '{}'", input);
        }
    }

    /// 顯示系統訊息
    pub fn system_message(&mut self, msg: &str) {
        self.window_manager.route_message("main", mudcore::window::WindowMessage {
            content: format!("\n[System] {}\n", msg),
            preserve_ansi: true,
            byte_widths: Vec::new(),
            repeat_count: 1,
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
    pub fn create_session(&mut self, profile: &Profile, api_mgr: &crate::api::ApiStateManager) -> SessionId {
        // 先產生 SessionId，用其 u64 值作為 session_key
        let session_id = SessionId::new();
        let session_key = session_id.value().to_string();
        
        // 在 ApiStateManager 中註冊並取得專屬的 SharedApiState
        let api_state = api_mgr.register_session(&session_key, &profile.display_name);
        
        let mut session = Session::from_profile(profile, api_state);
        session.id = session_id; // 覆寫為已產生的 ID
        session.merge_global_config(&self.global_aliases, &self.global_triggers);
        
        self.sessions.push(session);
        
        // 自動切換到新分頁
        self.active_index = self.sessions.len() - 1;
        
        // 設定為 active session
        api_mgr.set_active(&session_key);
        
        session_id
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

    /// 取得所有 Session 的可變參照
    pub fn sessions_mut(&mut self) -> &mut [Session] {
        &mut self.sessions
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
    use std::sync::{Arc, Mutex};
    use crate::config::{ConnectionConfig, Profile};

    #[test]
    fn test_session_id_unique() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_from_profile() {
        let api_state = Arc::new(Mutex::new(crate::api::ApiState::new("test_key".to_string(), "test".to_string())));
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
            notes: String::new(),
            paths: vec![],
        };

        let session = Session::from_profile(&profile, api_state);
        assert_eq!(session.profile_name, "test");
        assert_eq!(session.display_name, "測試");
        assert_eq!(session.host, "localhost");
    }

    #[test]
    fn test_session_manager_create_and_switch() {
        let api_mgr = crate::api::ApiStateManager::new();
        let mut manager = SessionManager::new();
        
        let profile1 = Profile::new("p1", "Profile 1")
            .with_connection("host1", "7777");
        let profile2 = Profile::new("p2", "Profile 2")
            .with_connection("host2", "7778");

        let id1 = manager.create_session(&profile1, &api_mgr);
        let id2 = manager.create_session(&profile2, &api_mgr);

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index(), 1); // 自動切到新分頁

        manager.switch_tab(0);
        assert_eq!(manager.active_session().unwrap().id, id1);

        manager.switch_tab(1);
        assert_eq!(manager.active_session().unwrap().id, id2);
        
        // 驗證兩個 session 有獨立的 API state
        let sessions = api_mgr.list_sessions();
        assert_eq!(sessions.len(), 2);
    }
}
