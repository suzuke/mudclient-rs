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
    TriggerManager, TriggerPattern, WindowManager,
};
use tokio::sync::mpsc;

use crate::config::{AliasConfig, Profile, TriggerConfig};

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
    Connect(String, u16),
    Send(String),
    Disconnect,
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
    
    /// 畫面單字字典（用於智慧補齊）
    pub screen_words: HashMap<String, WordMetadata>,
    
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

    // === 多視窗預留 ===
    /// 當 Session 被拆分為獨立視窗時的視窗 ID
    #[allow(dead_code)]
    pub detached_window_id: Option<u64>,
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

        // 載入 Profile 的別名
        for alias_cfg in &profile.aliases {
            let mut alias = Alias::new(&alias_cfg.name, &alias_cfg.pattern, &alias_cfg.replacement);
            alias.enabled = alias_cfg.enabled;
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
            status: ConnectionStatus::Disconnected,
            command_tx: None,
            message_rx: None,
            connected_at: None,
            alias_manager,
            trigger_manager,
            script_engine: ScriptEngine::new(),
            window_manager: WindowManager::new(),
            logger,
            input: String::new(),
            input_history: Vec::new(),
            history_index: None,
            tab_completion_prefix: None,
            tab_completion_index: 0,
            tab_completed: false,
            screen_words: HashMap::new(),
            in_room_description: false,
            auto_scroll: true,
            scroll_to_bottom_on_next_frame: false,
            auto_reconnect: true,
            reconnect_delay_until: None,
            detached_window_id: None,
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
            alias.enabled = alias_cfg.enabled;
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
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected(_))
    }

    /// 是否正在連線
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
