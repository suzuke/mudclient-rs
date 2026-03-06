//! Session 管理模組
//!
//! 每個 Session 代表一個獨立的 MUD 連線，擁有：
//! - 獨立的 Telnet 連線
//! - 獨立的觸發器/別名（從 Profile 載入）
//! - 獨立的訊息緩衝區與日誌
//!
//! SessionManager 管理所有活躍的 Session，並提供分頁切換功能。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use mudcore::{
    Alias, AliasManager, Logger, MapDatabase, ScriptEngine, Trigger, TriggerAction,
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
    /// Prompt 偵測：必須含有「數字/數字」樣式（如 hp2779/2779）
    static ref PROMPT_STAT_RE: regex::Regex = regex::Regex::new(r"\d+/\d+").unwrap();
}

/// Prompt 邊界標記 — 利用 telnet 協定中 prompt 沒有尾隨 \n 的特性
/// 當收到的 TCP chunk 不以 \n 結尾時，在 server_buffer 插入此標記
/// 作為 backward scan 的 stop signal，隔開廣播雜訊與房間輸出
const PROMPT_BOUNDARY: &str = "\x01PROMPT\x01";

// ============================================================================
// SessionId
// ============================================================================

/// Session 唯一識別碼
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

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

    /// 當前房間 ID 快取（避免每幀重算 SHA-256）
    pub current_room_id: Option<String>,

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
    pub input_history: std::collections::VecDeque<String>,
    
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

    /// MUD 指令字典（自動學習）
    pub command_dict: CommandDictionary,
    
    /// 是否正在接收房間敘述
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
    pub last_active: Instant,

    /// 活躍的計時器
    pub active_timers: Vec<ActiveTimer>,

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

    /// 事件匯流排
    pub event_bus: mudcore::EventBus,

    /// 內建地圖資料庫（Rust 原生 MudMapper）
    pub map_database: MapDatabase,
    /// 上次自動儲存時的 map data_version
    map_last_saved_version: u64,
}

/// 單字來源類型
#[derive(Debug, Clone, PartialEq)]
pub enum WordSource {
    /// 括號內或斜線後的 Mob/NPC/玩家 ID
    MobId,
    /// 房間敘述中的英文單字
    RoomDescription,
    /// 一般畫面單字
    ScreenText,
}

impl WordSource {
    /// 排序優先級（數字越小越優先）
    pub fn priority(&self) -> u8 {
        match self {
            WordSource::MobId => 0,
            WordSource::RoomDescription => 1,
            WordSource::ScreenText => 2,
        }
    }
}

/// 畫面單字的中繼資料
#[derive(Debug, Clone)]
pub struct WordMetadata {
    /// 最後一次出現的時間
    pub last_seen: Instant,
    /// 單字來源分類
    pub source: WordSource,
}

/// MUD 指令字典（自動學習 + 內建種子）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDictionary {
    /// 指令 -> 使用次數
    commands: HashMap<String, u32>,
}

impl CommandDictionary {
    /// 建立新字典並填入內建種子指令
    pub fn new() -> Self {
        let mut dict = Self {
            commands: HashMap::new(),
        };
        dict.load_seeds();
        dict
    }

    /// 內建種子指令（初始使用次數 = 0，作為基底候選）
    fn load_seeds(&mut self) {
        let seeds = [
            // 基本動作
            "look", "inventory", "score", "who", "say", "tell", "chat", "shout", "whisper",
            // 移動
            "north", "south", "east", "west", "up", "down",
            "northeast", "northwest", "southeast", "southwest",
            "enter", "leave", "recall", "follow",
            // 戰鬥
            "kill", "attack", "cast", "flee", "wimpy",
            // 物品
            "get", "drop", "put", "give", "wear", "remove", "wield", "unwield", "eat", "drink",
            // 資訊
            "help", "stat", "skills", "spells", "affects", "equipment", "examine",
            // 社交
            "bow", "smile", "laugh", "nod", "wave", "emote",
        ];
        for seed in seeds {
            self.commands.entry(seed.to_string()).or_insert(0);
        }
    }

    /// 記錄使用者輸入的指令（取第一個單字）
    pub fn record(&mut self, input: &str) {
        let cmd = input.split_whitespace().next().unwrap_or("").to_lowercase();
        if cmd.is_empty() || cmd.len() < 2 {
            return;
        }
        // 排除純數字和特殊字元開頭
        if cmd.chars().all(|c| c.is_ascii_digit()) {
            return;
        }
        let counter = self.commands.entry(cmd).or_insert(0);
        *counter = counter.saturating_add(1);
    }

    /// 回傳符合前綴的指令，依使用次數降序排列
    pub fn matches(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut results: Vec<_> = self.commands.iter()
            .filter(|(cmd, _)| cmd.starts_with(&prefix_lower))
            .collect();
        // 依使用次數降序，同次數按字母排序
        results.sort_by(|(a_cmd, a_count), (b_cmd, b_count)| {
            b_count.cmp(a_count)
                .then_with(|| a_cmd.cmp(b_cmd))
        });
        results.into_iter().map(|(cmd, _)| cmd.clone()).collect()
    }

    /// 從 JSON 檔案載入
    pub fn load(path: &std::path::Path) -> Self {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(mut dict) = serde_json::from_str::<CommandDictionary>(&content) {
                    // 確保種子指令存在（新版可能新增了種子）
                    dict.load_seeds();
                    return dict;
                }
            }
        }
        Self::new()
    }

    /// 儲存到 JSON 檔案
    pub fn save(&self, path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(path, content) {
                    tracing::error!("Failed to save command dictionary: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize command dictionary: {}", e);
            }
        }
    }

    /// 字典中的指令數量
    pub fn len(&self) -> usize {
        self.commands.len()
    }
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
            current_room_id: None,
            alias_manager,
            trigger_manager,
            path_manager,
            path_recorder: PathRecorder::new(),
            script_engine: ScriptEngine::new(),
            window_manager: WindowManager::new(),
            logger,
            input: String::new(),
            input_history: std::collections::VecDeque::new(),
            history_index: None,
            tab_completion_prefix: None,
            tab_completion_index: 0,
            tab_completed: false,
            last_completed_input: None,
            screen_words: HashMap::new(),
            command_dict: CommandDictionary::load(std::path::Path::new("data/command_dict.json")),
            in_room_description: false,
            auto_scroll: true,
            scroll_to_bottom_on_next_frame: false,
            auto_reconnect: true,
            reconnect_delay_until: None,
            last_active: Instant::now(),
            active_timers: Vec::new(),
            last_sent_command: None,
            repeat_command_count: 0,
            line_buffer: std::collections::VecDeque::with_capacity(20),
            server_buffer: std::collections::VecDeque::with_capacity(40),
            api_state,
            event_bus: mudcore::EventBus::new(),
            map_database: {
                let map_path = std::path::Path::new("data/mapper_data.json");
                if map_path.exists() {
                    MapDatabase::load_from_file(map_path).unwrap_or_else(|e| {
                        tracing::warn!("載入地圖資料庫失敗: {}", e);
                        MapDatabase::new()
                    })
                } else {
                    MapDatabase::new()
                }
            },
            map_last_saved_version: 0,
        };

        // 自動載入 scripts/ 目錄下的腳本
        session.load_startup_scripts();

        session
    }

    /// 從設定建立觸發器
    pub fn create_trigger_from_config(config: &TriggerConfig) -> Option<Trigger> {
        let clean_pattern = clean_pattern_string(&config.pattern);

        // 根據 pattern_type 決定匹配類型，None 時 fallback 到自動偵測
        let pattern = match config.pattern_type.as_deref() {
            Some("contains") => TriggerPattern::Contains(clean_pattern),
            Some("startswith") => TriggerPattern::StartsWith(clean_pattern),
            Some("endswith") => TriggerPattern::EndsWith(clean_pattern),
            Some("regex") => TriggerPattern::Regex(clean_pattern),
            _ => {
                // auto: 自動偵測正則表達式模式
                if clean_pattern.contains("(.+)")
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
                }
            }
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
        if let Some(ref g) = config.group {
            trigger = trigger.with_group(g.clone());
        }
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

    /// 處理來自 HTTP API 的查詢，回傳 JSON
    pub fn handle_api_query(&mut self, query: crate::api::ApiQuery) -> serde_json::Value {
        use crate::api::ApiQuery;
        match query {
            ApiQuery::ListAliases => {
                let aliases: Vec<serde_json::Value> = self.alias_manager.sorted_aliases.iter()
                    .filter_map(|name| self.alias_manager.get(name))
                    .map(|a| serde_json::json!({
                        "name": a.name,
                        "pattern": a.pattern,
                        "replacement": a.replacement,
                        "enabled": a.enabled,
                        "is_script": a.is_script,
                        "category": a.category,
                    }))
                    .collect();
                serde_json::json!({ "aliases": aliases, "count": aliases.len() })
            }
            ApiQuery::ListTriggers => {
                let triggers: Vec<serde_json::Value> = self.trigger_manager.order.iter()
                    .filter_map(|name| self.trigger_manager.get(name))
                    .map(|t| {
                        let (pat_str, pat_type) = match &t.pattern {
                            TriggerPattern::Contains(s) => (s.clone(), "contains"),
                            TriggerPattern::StartsWith(s) => (s.clone(), "startswith"),
                            TriggerPattern::EndsWith(s) => (s.clone(), "endswith"),
                            TriggerPattern::Regex(s) => (s.clone(), "regex"),
                        };
                        let action_str = t.actions.first().map(|a| match a {
                            TriggerAction::SendCommand(s) => s.clone(),
                            TriggerAction::ExecuteScript(s) => s.clone(),
                            _ => String::new(),
                        }).unwrap_or_default();
                        let is_script = t.actions.first().map(|a| matches!(a, TriggerAction::ExecuteScript(_))).unwrap_or(false);
                        serde_json::json!({
                            "name": t.name,
                            "pattern": pat_str,
                            "pattern_type": pat_type,
                            "action": action_str,
                            "enabled": t.enabled,
                            "is_script": is_script,
                            "category": t.category,
                        })
                    })
                    .collect();
                serde_json::json!({ "triggers": triggers, "count": triggers.len() })
            }
            ApiQuery::ListPaths => {
                let paths: Vec<serde_json::Value> = self.path_manager.sorted_keys.iter()
                    .filter_map(|name| self.path_manager.get(name))
                    .map(|p| serde_json::json!({
                        "name": p.name,
                        "value": p.value,
                        "category": p.category,
                    }))
                    .collect();
                serde_json::json!({ "paths": paths, "count": paths.len() })
            }
            ApiQuery::ListWindows => {
                let windows: Vec<serde_json::Value> = self.window_manager.windows().iter()
                    .map(|w| serde_json::json!({
                        "id": w.id,
                        "title": w.title,
                        "visible": w.visible,
                        "message_count": w.message_count(),
                    }))
                    .collect();
                serde_json::json!({ "windows": windows })
            }
            ApiQuery::ReadWindow { id, count } => {
                match self.window_manager.get(&id) {
                    Some(window) => {
                        let messages: Vec<String> = window.last_n(count)
                            .map(|m| {
                                if m.content.contains('\x1b') {
                                    ANSI_STRIP_RE.replace_all(&m.content, "").to_string()
                                } else {
                                    m.content.clone()
                                }
                            })
                            .collect();
                        serde_json::json!({
                            "id": id,
                            "messages": messages,
                            "count": messages.len(),
                        })
                    }
                    None => serde_json::json!({ "error": format!("Window '{}' not found", id) }),
                }
            }
            ApiQuery::GetHistory { count } => {
                let total = self.input_history.len();
                let start = total.saturating_sub(count);
                let history: Vec<&str> = self.input_history.iter().skip(start).map(|s| s.as_str()).collect();
                serde_json::json!({ "history": history, "total": total })
            }
            ApiQuery::GetMapStats => {
                serde_json::json!({
                    "enabled": self.map_database.enabled,
                    "room_count": self.map_database.rooms.len(),
                    "edge_count": self.map_database.edge_count(),
                    "current_room_id": self.current_room_id,
                    "last_room_id": self.map_database.last_room_id,
                })
            }
            ApiQuery::SearchMapRooms { query } => {
                let results = self.map_database.resolve_target(&query);
                let rooms: Vec<serde_json::Value> = results.iter()
                    .take(20)
                    .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
                    .collect();
                serde_json::json!({ "results": rooms, "total": results.len() })
            }
            ApiQuery::FindMapPath { from, to } => {
                // 支援 "current" 關鍵字
                let from_id = if from == "current" {
                    self.current_room_id.clone().unwrap_or_default()
                } else {
                    // 嘗試 resolve 名稱到 ID
                    self.map_database.resolve_target(&from)
                        .first().map(|(id, _)| id.clone()).unwrap_or(from)
                };
                let to_targets = self.map_database.resolve_target(&to);
                let to_id = to_targets.first().map(|(id, _)| id.clone()).unwrap_or(to);

                if from_id.is_empty() {
                    return serde_json::json!({ "error": "Cannot determine source room" });
                }
                match self.map_database.find_path(&from_id, &to_id) {
                    Some(path) => serde_json::json!({
                        "found": true,
                        "from": from_id,
                        "to": to_id,
                        "path": path,
                        "steps": path.len(),
                    }),
                    None => serde_json::json!({
                        "found": false,
                        "from": from_id,
                        "to": to_id,
                    }),
                }
            }
            ApiQuery::ManageAlias { action, name, pattern, replacement, is_script } => {
                match action.as_str() {
                    "add" => {
                        let pat = pattern.unwrap_or_default();
                        let rep = replacement.unwrap_or_default();
                        let mut alias = mudcore::Alias::new(&name, &pat, &rep);
                        alias.is_script = is_script.unwrap_or(false);
                        self.alias_manager.add(alias);
                        serde_json::json!({ "ok": true, "message": format!("Alias '{}' added", name) })
                    }
                    "remove" => {
                        match self.alias_manager.remove(&name) {
                            Some(_) => serde_json::json!({ "ok": true, "message": format!("Alias '{}' removed", name) }),
                            None => serde_json::json!({ "ok": false, "message": format!("Alias '{}' not found", name) }),
                        }
                    }
                    "toggle" => {
                        if let Some(a) = self.alias_manager.aliases.get_mut(&name) {
                            a.enabled = !a.enabled;
                            serde_json::json!({ "ok": true, "enabled": a.enabled })
                        } else {
                            serde_json::json!({ "ok": false, "message": format!("Alias '{}' not found", name) })
                        }
                    }
                    _ => serde_json::json!({ "ok": false, "message": format!("Unknown action '{}'", action) }),
                }
            }
            ApiQuery::ManageTrigger { action, name, pattern, pattern_type, script } => {
                match action.as_str() {
                    "add" => {
                        let pat_str = pattern.unwrap_or_default();
                        let pat = match pattern_type.as_deref() {
                            Some("startswith") => TriggerPattern::StartsWith(pat_str),
                            Some("endswith") => TriggerPattern::EndsWith(pat_str),
                            Some("regex") => TriggerPattern::Regex(pat_str),
                            _ => TriggerPattern::Contains(pat_str),
                        };
                        let mut trigger = mudcore::Trigger::new(&name, pat);
                        if let Some(s) = script {
                            trigger = trigger.add_action(TriggerAction::ExecuteScript(s));
                        }
                        self.trigger_manager.add(trigger);
                        serde_json::json!({ "ok": true, "message": format!("Trigger '{}' added", name) })
                    }
                    "remove" => {
                        match self.trigger_manager.remove(&name) {
                            Some(_) => serde_json::json!({ "ok": true, "message": format!("Trigger '{}' removed", name) }),
                            None => serde_json::json!({ "ok": false, "message": format!("Trigger '{}' not found", name) }),
                        }
                    }
                    "toggle" => {
                        if let Some(t) = self.trigger_manager.triggers.get_mut(&name) {
                            t.enabled = !t.enabled;
                            serde_json::json!({ "ok": true, "enabled": t.enabled })
                        } else {
                            serde_json::json!({ "ok": false, "message": format!("Trigger '{}' not found", name) })
                        }
                    }
                    _ => serde_json::json!({ "ok": false, "message": format!("Unknown action '{}'", action) }),
                }
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
        let scan_limit = 30; // 容納長描述房間 (如 14+ 行的魔幻空間)
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

            // 0. Prompt boundary (telnet 協定層偵測 — prompt 沒有尾隨 \n)
            // 這是最可靠的分界符，不依賴 prompt 內容
            if trim_line == PROMPT_BOUNDARY {
                name_index = i + 1;
                found_prev_exit = true;
                break;
            }

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
            
            // 5. Mob/NPC/玩家存在行 (例如 "XXX 站在這兒.")
            // 這些行出現在出口行之後，屬於上一個房間區塊的殘留
            if trim_line.ends_with("站在這兒.")
                || trim_line.ends_with("站在這裡.")
                || trim_line.ends_with("is here.")
                || trim_line.ends_with("is standing here.")
            {
                name_index = i + 1;
                found_prev_exit = true;
                break;
            }
        }
        
        // 沒有可靠的分隔線 → 放棄偵測
        // 根因：登入後首次偵測時 buffer 充滿 MOTD/登入文字，無錨點會導致 hash 不穩定
        // 此外，非房間指令（如 ch）的輸出若包含 [出口:] 也會在無錨點時誤判
        if !found_prev_exit {
            return;
        }
        
        if name_index >= n - 1 {
            // 沒有中間內容 (例如連續兩個出口行? 不太可能，除非快速移動且 gag 了描述)
            return;
        }
        
        // [向前掃描] 從 name_index 開始，跳過非房間名的雜訊行
        // 觀察：MUD 房間名（如「市中心」、「風采裝備倉庫」）不以句末標點結尾
        //       而 NPC 動作/玩家行為/系統訊息 都以句末標點結尾
        while name_index < n - 1 {
            let raw = &self.server_buffer[name_index];
            let clean = if raw.contains('\x1b') {
                ANSI_STRIP_RE.replace_all(raw, "").to_string()
            } else {
                raw.to_string()
            };
            let trimmed = clean.trim();
            
            // 跳過明確不是房間名的行：
            // - 空行
            // - 以句末標點結尾（NPC 動作/系統訊息）
            // - 以 '(' 開頭（NPC 光環標記如「(白色聖光) NPC名」，房間名不會以括號開頭）
            // - 以 '>' 開頭（指令回顯）
            let ends_with_punctuation = trimmed.ends_with('.')
                || trimmed.ends_with('。')
                || trimmed.ends_with('?')
                || trimmed.ends_with('？')
                || trimmed.ends_with('!')
                || trimmed.ends_with('！');
            let starts_with_noise = trimmed.starts_with('(') || trimmed.starts_with('>');
            let is_boundary = trimmed == PROMPT_BOUNDARY;
            if trimmed.is_empty() || ends_with_punctuation || starts_with_noise || is_boundary {
                name_index += 1;
            } else {
                break;
            }
        }
        
        if name_index >= n - 1 {
            return; // 全部都是雜訊行，放棄偵測
        }
        
        // 取得名稱 (移除 ANSI)
        let raw_name = &self.server_buffer[name_index];
        let name = if raw_name.contains('\x1b') {
            ANSI_STRIP_RE.replace_all(raw_name, "").to_string()
        } else {
            raw_name.to_string()
        };
        let name = name.trim().to_string();

        // 防禦性檢查：名稱不能是出口行本身（如 ch 指令輸出意外觸發偵測）
        if name.starts_with("[出口:") || name.starts_with("[Exits:") || name.is_empty() {
            return;
        }

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
            
            // 偵測 Mob/NPC/玩家存在行
            let is_presence = trimmed.ends_with("站在這兒.")
                || trimmed.ends_with("站在這裡.")
                || trimmed.ends_with("is here.")
                || trimmed.ends_with("is standing here.");
            
            let is_boundary = trimmed == PROMPT_BOUNDARY;
            let is_noise = is_prompt || is_script || is_presence || is_boundary;
            
            if !is_noise {
                desc_lines.push(clean_line);
            } else {
                tracing::debug!("Excluding noise from room hash: {}", trimmed);
            }
        }
        let description = desc_lines.join("\n");
        
        // 更新 API 共享狀態的房間資訊（先取用 exits，避免多次 clone）
        let api_exits = exits.clone();

        // 建立 Room 物件（消耗 exits）
        let room = Room::new(&name, &description, exits);
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

        self.script_engine.set_current_room(Some(room.clone()));
        self.current_room = Some(room);
        self.current_room_id = Some(id.clone());

        // Rust 內建地圖記錄
        self.map_database.on_room_detected(&id, &name, &api_exits);

        // 自動儲存地圖（每 10 次變更）
        if self.map_database.data_version - self.map_last_saved_version >= 10 {
            let path = std::path::Path::new("data/mapper_data.json");
            if let Err(e) = self.map_database.save(path) {
                tracing::warn!("[Map] 自動儲存失敗: {}", e);
            }
            self.map_last_saved_version = self.map_database.data_version;
        }

        if let Ok(mut api) = self.api_state.lock() {
            api.current_room = Some(crate::api::RoomInfo {
                name: name.clone(),
                exits: api_exits,
                room_id: Some(id.clone()),
                description: description.clone(),
            });
        }
        
        // 觸發 Lua Hook: on_room_detected(id, name)
        // 這裡我們傳入預設的 strict ID，但 Lua 腳本可以自己呼叫 mud.get_current_room() 取得更多資訊
        // 或者 mud.get_current_room_id(false) 取得 lax ID
        // 觸發 Lua Hook: on_room_detected(id, name)
        match self.script_engine.invoke_hook("on_room_detected", &id, &name, false) {
            Ok(Some(context)) => {
                self.apply_script_context(context);
            }
            Ok(None) => {}, // Hook not defined
            Err(e) => {
                 tracing::error!("on_room_detected hook error: {}", e);
            }
        }

        // Emit room_changed event
        let event_data = format!(r#"{{"id":"{}","name":"{}"}}"#, id, name.replace('"', "\\\""));
        self.emit_event("room_changed", Some(event_data));

        // [結構性修復] 偵測成功後清空 server_buffer，保留出口行作為下次的錨點
        // 這樣出口之後的 NPC/玩家行不會殘留在 buffer 中，汙染下次房間偵測
        self.server_buffer.clear();
        self.server_buffer.push_back(clean_current);
    }

    /// 檢查並執行到期的計時器
    pub fn check_timers(&mut self) {
        if self.active_timers.is_empty() {
            return;
        }

        let now = Instant::now();
        let mut expired = Vec::new();

        let mut i = 0;
        while i < self.active_timers.len() {
            if now >= self.active_timers[i].expires_at {
                let timer = self.active_timers.swap_remove(i);
                expired.push(timer.lua_code); // move 而非 clone
            } else {
                i += 1;
            }
        }

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
                        let resources_dir = contents_dir.join("Resources");
                        let res_scripts = resources_dir.join("scripts");
                        if res_scripts.exists() && res_scripts.is_dir() {
                            // chdir 到 Resources，讓 Lua 相對路徑 (data/, logs/) 正確解析
                            if let Err(e) = std::env::set_current_dir(&resources_dir) {
                                tracing::warn!("無法切換工作目錄到 Resources: {}", e);
                            } else {
                                tracing::info!("工作目錄已切換至: {}", resources_dir.display());
                            }
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
        // [修正] 直接路由到視窗，不經過 handle_text，避免 handle_text → invoke_hook →
        // mud.echo → apply_script_context → handle_text 的間接遞迴導致 stack overflow
        for echo in context.echos {
            self.window_manager.route_message(
                "main",
                WindowMessage {
                    content: echo.clone(),
                    preserve_ansi: true,
                    byte_widths: Vec::new(),
                    repeat_count: 1,
                },
            );
            // 日誌記錄 echo
            let _ = self.logger.log(&echo);
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

        // 6b. Trigger group updates
        for (group, enabled) in context.group_updates {
            let count = self.trigger_manager.enable_group(&group, enabled);
            if count > 0 {
                tracing::info!("Script updated trigger group '{}' enabled={}: {} triggers affected", group, enabled, count);
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

        // 9. LLM 請求（非同步呼叫 Anthropic API）
        for req in context.llm_requests {
            self.dispatch_llm_request(req);
        }

        // 10. Event handler registrations
        for (name, code, priority, once) in context.event_registrations {
            self.event_bus.on(&name, code, priority, once);
        }

        // 11. Event handler removals
        for id in context.event_removals {
            self.event_bus.off(id);
        }

        // 12. Event emissions
        for (name, data) in context.event_emissions {
            self.emit_event(&name, data);
        }
    }

    /// 觸發事件並執行所有 handler
    pub fn emit_event(&mut self, event_name: &str, data: Option<String>) {
        let handlers = self.event_bus.emit(event_name, data.clone());
        for (_id, lua_code) in handlers {
            // Set event_data global before executing handler
            let setup = if let Some(ref d) = data {
                format!("event_data = [==[{}]==]", d)
            } else {
                "event_data = nil".to_string()
            };
            let full_code = format!("{}\n{}", setup, lua_code);
            match self.script_engine.execute_inline(&full_code, "", &[], false) {
                Ok(ctx) => self.apply_script_context(ctx),
                Err(e) => {
                    tracing::error!("Event handler error for '{}': {}", event_name, e);
                }
            }
        }
    }

    /// 派發 LLM 請求：透過 api_state 排入 pending_lua，由 tokio 非同步執行
    fn dispatch_llm_request(&self, req: mudcore::LlmRequest) {
        let api_state = self.api_state.clone();

        let llm_future = async move {
                let api_key = match std::env::var("ANTHROPIC_API_KEY") {
                    Ok(k) if !k.is_empty() => k,
                    _ => {
                        if let Ok(mut s) = api_state.lock() {
                            s.pending_lua.push_back(
                                r#"mud.echo("\x1b[31m[LLM] ANTHROPIC_API_KEY not set\x1b[0m")"#.to_string()
                            );
                        }
                        return;
                    }
                };

                let model = req.model.as_deref().unwrap_or("claude-haiku-4-5-20251001");

                let client = reqwest::Client::new();
                let body = serde_json::json!({
                    "model": model,
                    "max_tokens": 256,
                    "messages": [{"role": "user", "content": req.prompt}]
                });

                let resp = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await;

                let result_text = match resp {
                    Ok(r) => {
                        match r.json::<serde_json::Value>().await {
                            Ok(json) => {
                                json["content"]
                                    .as_array()
                                    .and_then(|arr| arr.first())
                                    .and_then(|c| c["text"].as_str())
                                    .unwrap_or("")
                                    .trim()
                                    .to_string()
                            }
                            Err(e) => {
                                tracing::error!("[LLM] JSON parse error: {}", e);
                                String::new()
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("[LLM] HTTP error: {}", e);
                        String::new()
                    }
                };

                if result_text.is_empty() {
                    if let Ok(mut s) = api_state.lock() {
                        s.pending_lua.push_back(
                            r#"mud.echo("\x1b[31m[LLM] Empty or failed response\x1b[0m")"#.to_string()
                        );
                    }
                    return;
                }

                // 將 $RESULT 替換為 LLM 回覆（JSON escape 避免注入）
                let escaped = result_text
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "");
                let lua_code = req.callback_code.replace("$RESULT", &format!("\"{}\"", escaped));

                if let Ok(mut s) = api_state.lock() {
                    s.pending_lua.push_back(lua_code);
                }
        };

        // 優先使用現有 tokio runtime，避免每次建立新 thread + runtime
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(llm_future);
        } else {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(llm_future);
                }
            });
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
            for line in &lines {
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

            // [Prompt Boundary] Telnet 協定：prompt 不以 \n 結尾
            // 若整段 text 不以 \n 結尾，代表最後一段是 prompt → 插入邊界標記
            // 這讓 backward scan 能正確隔開「廣播雜訊」與「房間輸出」
            if !is_echo && !text.ends_with('\n') {
                if self.server_buffer.len() >= 40 {
                    self.server_buffer.pop_front();
                }
                self.server_buffer.push_back(PROMPT_BOUNDARY.to_string());
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
            let is_prompt_pre = clean_lower_pre.trim().starts_with('(') && clean_lower_pre.contains(')') && PROMPT_STAT_RE.is_match(&clean_lower_pre);
            if !is_prompt_pre {
                if self.server_buffer.len() >= 40 {
                    self.server_buffer.pop_front();
                }
                self.server_buffer.push_back(text.trim().to_string());
                
                // 嘗試偵測房間 (改用純淨緩衝區)
                self.detect_room_info(text.trim());
            }
        }

        // 0. 呼叫全域鉤子 (Global Hook)
        // 這允許 Lua 腳本直接處理每一行伺服器訊息與回顯，無需透過正則表達式觸發器
        match self.script_engine.invoke_hook("on_server_message", text, &clean_text, is_echo) {
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
            // 處理觸發器（使用已 strip ANSI 的 clean_text 避免重複 strip）
            let triggers = self.trigger_manager.process_pre_stripped(&clean_text);
            
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
        // 優化提示字元偵測：不分大小寫，先 trim 避免前導空白/CR 干擾
        // 必須同時滿足：1) 以 ( 開頭 2) 含 ) 3) 含有「數字/數字」樣式
        // 這樣可以正確區分 prompt 與物品行如 (閃爍) .../Silver Bow
        let trimmed_lower = clean_lower.trim();
        let is_prompt = trimmed_lower.starts_with('(') && trimmed_lower.contains(')') && PROMPT_STAT_RE.is_match(trimmed_lower);

        // 如果是房間敘述，且非出口行、非 Prompt、非 Echo，則進行標點轉換
        let is_exit_line = clean_text.contains("[出口:");
        
        // 房間敘述期間去除殘留 \r，避免複製貼上和 log 產生多餘空行
        let (final_text, final_widths) = if self.in_room_description {
            let stripped = text.trim_matches('\r');
            let widths = if let Some(bw) = byte_widths {
                // 去掉頭尾 \r 對應的寬度
                let leading_cr = text.len() - text.trim_start_matches('\r').len();
                let trailing_cr = text.len() - text.trim_end_matches('\r').len();
                let char_count = bw.len();
                let start = leading_cr.min(char_count);
                let end = char_count.saturating_sub(trailing_cr).max(start);
                bw[start..end].to_vec()
            } else {
                stripped.chars().map(|ch| if ch.is_ascii() { 1u8 } else { 2u8 }).collect()
            };
            (stripped.to_string(), widths)
        } else {
            let widths = if let Some(bw) = byte_widths {
                bw.to_vec()
            } else {
                text.chars().map(|ch| if ch.is_ascii() { 1u8 } else { 2u8 }).collect()
            };
            (text.to_string(), widths)
        };

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
        let preserve_ansi = !is_echo;
        for target_id in &targets {
            self.window_manager.route_message_with_widths(
                target_id,
                WindowMessage {
                    content: final_text.clone(),
                    preserve_ansi,
                    byte_widths: final_widths.clone(),
                    repeat_count: 1,
                },
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

        // 從所有非 prompt、非 echo 的伺服器訊息中提取英文單字
        if !is_prompt && !is_echo {
            let now = Instant::now();
            
            // 1. 特殊提取：括號內的內容 → MobId（優先級最高）
            if has_mob_brackets {
                for cap in MOB_BRACKET_RE.captures_iter(&clean_text) {
                    let content = &cap[1];
                    for word in content.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
                        if word.len() >= 2
                            && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                            && !word.chars().all(|c| c.is_ascii_digit())
                            && !is_stat_word(word)
                        {
                            self.screen_words.insert(word.to_string(), WordMetadata {
                                last_seen: now,
                                source: WordSource::MobId,
                            });
                        }
                    }
                }
            }

            // 2. 特殊提取：斜線後的內容 → MobId（針對 "中文/ID" 格式）
            if is_slash_line {
                if let Some(slash_idx) = clean_text.rfind('/') {
                    let after_slash = &clean_text[slash_idx+1..];
                    for word in after_slash.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
                        if word.len() >= 2
                            && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                            && !word.chars().all(|c| c.is_ascii_digit())
                            && !is_stat_word(word)
                        {
                            self.screen_words.insert(word.to_string(), WordMetadata {
                                last_seen: now,
                                source: WordSource::MobId,
                            });
                        }
                    }
                }
            }

            // 3. 通用提取：整行所有英文單字
            let source = if self.in_room_description {
                WordSource::RoomDescription
            } else {
                WordSource::ScreenText
            };
            for word in clean_text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
                if word.len() >= 2
                    && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                    && !word.chars().all(|c| c.is_ascii_digit())
                    && !is_stat_word(word)
                {
                    let entry = self.screen_words.entry(word.to_string()).or_insert(WordMetadata {
                        last_seen: now,
                        source: source.clone(),
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

        // 日誌記錄（使用已處理過的 final_text）
        let _ = self.logger.log(&final_text);

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
        // 沒有 callback 時不需要執行腳本（send_chain 的中間指令）
        if callback_code.is_empty() {
            tracing::debug!("CollectedResponse: {} lines, no callback (chain intermediate)", lines.len());
            return;
        }

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
                            let lua_code = format!("mud.send(\"{}\")", sub_cmd.replace('\\', "\\\\").replace('"', "\\\""));
                            
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
                "#map" => {
                    if parts.len() < 2 {
                        self.system_message("Usage: #map <start|stop|save|load|status|find|path|go>");
                        return;
                    }
                    match parts[1] {
                        "start" => {
                            self.map_database.enable();
                            self.system_message("[Map] 地圖記錄已啟用。");
                        }
                        "stop" => {
                            self.map_database.disable();
                            self.system_message("[Map] 地圖記錄已停用。");
                        }
                        "save" => {
                            let path = std::path::Path::new("data/mapper_data.json");
                            match self.map_database.save(path) {
                                Ok(()) => self.system_message(&format!(
                                    "[Map] 地圖已儲存 ({} 房間, {} 邊緣)",
                                    self.map_database.rooms.len(),
                                    self.map_database.edge_count()
                                )),
                                Err(e) => self.system_message(&format!("[Map] 儲存失敗: {}", e)),
                            }
                        }
                        "load" => {
                            let path = std::path::Path::new("data/mapper_data.json");
                            match MapDatabase::load_from_file(path) {
                                Ok(db) => {
                                    let room_count = db.rooms.len();
                                    let edge_count = db.edge_count();
                                    self.map_database = db;
                                    self.system_message(&format!(
                                        "[Map] 已載入地圖 ({} 房間, {} 邊緣)",
                                        room_count, edge_count
                                    ));
                                }
                                Err(e) => self.system_message(&format!("[Map] 載入失敗: {}", e)),
                            }
                        }
                        "status" => {
                            let current = self.map_database.last_room_id.as_deref()
                                .and_then(|id| self.map_database.rooms.get(id))
                                .map(|r| r.name.as_str())
                                .unwrap_or("未知");
                            self.system_message(&format!(
                                "[Map] 狀態: {} | 房間: {} | 邊緣: {} | 當前: {}",
                                if self.map_database.enabled { "啟用" } else { "停用" },
                                self.map_database.rooms.len(),
                                self.map_database.edge_count(),
                                current
                            ));
                        }
                        "find" => {
                            if parts.len() < 3 {
                                self.system_message("Usage: #map find <名稱或ID>");
                                return;
                            }
                            let query = parts[2..].join(" ");
                            let results = self.map_database.resolve_target(&query);
                            if results.is_empty() {
                                self.system_message(&format!("[Map] 找不到符合 '{}' 的房間。", query));
                            } else {
                                self.system_message(&format!("[Map] 找到 {} 個結果:", results.len()));
                                for (id, name) in &results {
                                    let short_id = if id.len() > 12 { &id[..12] } else { id };
                                    self.system_message(&format!("  {} ({}...)", name, short_id));
                                }
                            }
                        }
                        "path" => {
                            if parts.len() < 3 {
                                self.system_message("Usage: #map path <目標>");
                                return;
                            }
                            let query = parts[2..].join(" ");
                            let from = match &self.map_database.last_room_id {
                                Some(id) => id.clone(),
                                None => {
                                    self.system_message("[Map] 尚未定位，請先移動。");
                                    return;
                                }
                            };
                            let targets = self.map_database.resolve_target(&query);
                            match targets.len() {
                                0 => self.system_message(&format!("[Map] 找不到 '{}'。", query)),
                                1 => {
                                    let (to_id, to_name) = &targets[0];
                                    match self.map_database.find_path(&from, to_id) {
                                        Some(path) if path.is_empty() => {
                                            self.system_message("[Map] 你已在目的地。");
                                        }
                                        Some(path) => {
                                            let path_str = path.join(";");
                                            self.system_message(&format!(
                                                "[Map] 路徑至 {} ({} 步): {}",
                                                to_name, path.len(), path_str
                                            ));
                                        }
                                        None => self.system_message(&format!(
                                            "[Map] 找不到通往 {} 的路徑。", to_name
                                        )),
                                    }
                                }
                                _ => {
                                    self.system_message(&format!("[Map] 目標不明確，找到 {} 個結果:", targets.len()));
                                    for (id, name) in &targets {
                                        let short_id = if id.len() > 12 { &id[..12] } else { id };
                                        self.system_message(&format!("  {} ({}...)", name, short_id));
                                    }
                                }
                            }
                        }
                        "go" => {
                            if parts.len() < 3 {
                                self.system_message("Usage: #map go <目標>");
                                return;
                            }
                            let query = parts[2..].join(" ");
                            let from = match &self.map_database.last_room_id {
                                Some(id) => id.clone(),
                                None => {
                                    self.system_message("[Map] 尚未定位，請先移動。");
                                    return;
                                }
                            };
                            let targets = self.map_database.resolve_target(&query);
                            match targets.len() {
                                0 => self.system_message(&format!("[Map] 找不到 '{}'。", query)),
                                1 => {
                                    let (to_id, to_name) = &targets[0];
                                    match self.map_database.find_path(&from, to_id) {
                                        Some(path) if path.is_empty() => {
                                            self.system_message("[Map] 你已在目的地。");
                                        }
                                        Some(path) => {
                                            self.system_message(&format!(
                                                "[Map] 導航至 {} ({} 步)...",
                                                to_name, path.len()
                                            ));
                                            for dir in path {
                                                self.handle_user_input_with_depth(&dir, depth + 1);
                                            }
                                        }
                                        None => self.system_message(&format!(
                                            "[Map] 找不到通往 {} 的路徑。", to_name
                                        )),
                                    }
                                }
                                _ => {
                                    self.system_message(&format!("[Map] 目標不明確，找到 {} 個結果:", targets.len()));
                                    for (id, name) in &targets {
                                        let short_id = if id.len() > 12 { &id[..12] } else { id };
                                        self.system_message(&format!("  {} ({}...)", name, short_id));
                                    }
                                }
                            }
                        }
                        _ => {
                            self.system_message("Usage: #map <start|stop|save|load|status|find|path|go>");
                        }
                    }
                    return;
                }
                _ => {
                    // 如果不是已知指令，則視為普通內容發送
                }
            }
        }

        // 記錄使用者指令到字典（排除客戶端指令）
        if !input.starts_with('#') && !input.starts_with('/') {
            self.command_dict.record(&input);
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

            // Rust 內建地圖：記錄移動方向
            self.map_database.record_last_direction(&input);

            // 觸發 Lua Hook: on_command(command)
            if let Ok(Some(context)) = self.script_engine.invoke_hook("on_command", &input, &input, false) {
                 self.apply_script_context(context);
            }

            // Emit command_sent event
            let event_data = format!(r#"{{"command":"{}"}}"#, input.replace('\\', "\\\\").replace('"', "\\\""));
            self.emit_event("command_sent", Some(event_data));

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
            // 關閉前自動儲存地圖
            if self.sessions[pos].map_database.data_version > self.sessions[pos].map_last_saved_version {
                let path = std::path::Path::new("data/mapper_data.json");
                if let Err(e) = self.sessions[pos].map_database.save(path) {
                    tracing::warn!("[Map] 關閉 session 時儲存地圖失敗: {}", e);
                }
            }
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

/// 判斷單字是否為 Prompt 中的 stat 縮寫+數字模式
/// 例如: "hp2779", "ma67", "v1364", "p311"
/// 規則: 1~3 個字母開頭 + 純數字結尾
fn is_stat_word(word: &str) -> bool {
    if word.len() < 2 {
        return false;
    }
    let letter_count = word.chars().take_while(|c| c.is_ascii_alphabetic()).count();
    if letter_count == 0 || letter_count > 3 {
        return false; // 沒有字母前綴，或前綴超過 3 字母（可能是正常單字如 "guard2"）
    }
    let rest = &word[letter_count..];
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
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

    #[test]
    fn test_command_dictionary_record_and_match() {
        let mut dict = CommandDictionary::new();
        
        // 記錄一些指令
        dict.record("kill warrior");
        dict.record("kill goblin");
        dict.record("kill warrior");
        dict.record("kick someone");
        
        // 查詢 "ki" 前綴
        let results = dict.matches("ki");
        assert!(results.contains(&"kill".to_string()));
        assert!(results.contains(&"kick".to_string()));
        // kill 使用次數 = 2，應排在 kick (1) 前面
        assert_eq!(results[0], "kill");
    }

    #[test]
    fn test_command_dictionary_seeds() {
        let dict = CommandDictionary::new();
        
        // 種子指令應存在
        let results = dict.matches("loo");
        assert!(results.contains(&"look".to_string()));
        
        let results = dict.matches("ki");
        assert!(results.contains(&"kill".to_string()));
        
        let results = dict.matches("inv");
        assert!(results.contains(&"inventory".to_string()));
    }

    #[test]
    fn test_command_dictionary_excludes_pure_digits() {
        let mut dict = CommandDictionary::new();
        dict.record("123");
        dict.record("n");  // 太短，長度 < 2
        
        // 純數字和太短的不該被記錄
        assert!(dict.matches("123").is_empty());
        assert!(dict.matches("n").iter().all(|c| c != "n"));
    }

    #[test]
    fn test_command_dictionary_persistence() {
        let tmp_path = std::path::Path::new("/tmp/test_cmd_dict.json");
        
        // 建立並記錄
        let mut dict = CommandDictionary::new();
        dict.record("mycommand arg1");
        dict.record("mycommand arg2");
        dict.save(tmp_path);
        
        // 重新載入
        let loaded = CommandDictionary::load(tmp_path);
        let results = loaded.matches("myc");
        assert!(results.contains(&"mycommand".to_string()));
        
        // 清理
        let _ = std::fs::remove_file(tmp_path);
    }

    #[test]
    fn test_word_source_priority() {
        assert!(WordSource::MobId.priority() < WordSource::RoomDescription.priority());
        assert!(WordSource::RoomDescription.priority() < WordSource::ScreenText.priority());
    }

    #[test]
    fn test_word_metadata_with_source() {
        let meta = WordMetadata {
            last_seen: Instant::now(),
            source: WordSource::MobId,
        };
        assert_eq!(meta.source, WordSource::MobId);
    }

    #[test]
    fn test_is_stat_word() {
        // Prompt stat 樣式（應排除）
        assert!(is_stat_word("hp2779"));
        assert!(is_stat_word("ma67"));
        assert!(is_stat_word("v1364"));
        assert!(is_stat_word("p311"));
        assert!(is_stat_word("sp50"));
        
        // 正常 MUD 單字（不應排除）
        assert!(!is_stat_word("warrior"));      // 純字母
        assert!(!is_stat_word("guard2"));        // 4 字母前綴 > 3
        assert!(!is_stat_word("necklace"));      // 純字母
        assert!(!is_stat_word("12345"));         // 純數字（由其他過濾處理）
        assert!(!is_stat_word("a"));             // 太短
    }
}
