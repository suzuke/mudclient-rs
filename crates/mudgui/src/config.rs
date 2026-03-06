//! 設定檔持久化模組
//!
//! 支援多 Profile 與全域設定的架構：
//! - `GlobalConfig`: 全域設定（全域別名/觸發器、UI 偏好、自動連線列表）
//! - `Profile`: 單一帳號/伺服器的設定（連線資訊、專屬別名/觸發器）
//! - `ProfileManager`: Profile 的 CRUD 操作

use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn ser_password<S: Serializer>(pw: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
    match pw {
        Some(p) => s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(p)),
        None => s.serialize_none(),
    }
}

fn de_password<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let opt: Option<String> = Option::deserialize(d)?;
    Ok(opt.and_then(|encoded| {
        base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }))
}

// ============================================================================
// 基礎設定結構（與舊版相容）
// ============================================================================

/// 別名設定（可序列化版本）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AliasConfig {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub is_script: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// 觸發器設定（可序列化版本）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TriggerConfig {
    pub name: String,
    pub pattern: String,
    pub action: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub is_script: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 匹配類型: "auto", "contains", "startswith", "endswith", "regex"
    #[serde(default)]
    pub pattern_type: Option<String>,
}

/// 路徑設定（可序列化版本）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathConfig {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub category: Option<String>,
}

/// 連線設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: String,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// 新架構：Profile
// ============================================================================

/// 單一帳號/伺服器的完整設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile 識別名稱（用於檔名，僅限 ASCII）
    pub name: String,
    /// 顯示名稱（可含中文）
    #[serde(default)]
    pub display_name: String,
    /// 連線資訊
    pub connection: ConnectionConfig,
    /// Profile 專屬別名
    #[serde(default)]
    pub aliases: Vec<AliasConfig>,
    /// Profile 專屬觸發器
    #[serde(default)]
    pub triggers: Vec<TriggerConfig>,
    /// Profile 專屬路徑
    #[serde(default)]
    pub paths: Vec<PathConfig>,
    /// 自訂腳本載入路徑（可選）
    #[serde(default)]
    pub script_paths: Vec<String>,
    
    /// 用戶筆記
    #[serde(default)]
    pub notes: String,
    
    // === 帳號資訊 ===
    /// 登入帳號
    #[serde(default)]
    pub username: Option<String>,
    /// 登入密碼（base64 編碼儲存）
    #[serde(default, skip_serializing_if = "Option::is_none",
            serialize_with = "ser_password", deserialize_with = "de_password")]
    pub password: Option<String>,

    /// 建立時間 (Unix timestamp)
    #[serde(default)]
    pub created_at: u64,
    /// 最後連線時間 (Unix timestamp)
    #[serde(default)]
    pub last_connected: Option<u64>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            display_name: "預設".to_string(),
            connection: ConnectionConfig::default(),
            aliases: Vec::new(),
            triggers: Vec::new(),
            paths: Vec::new(),
            script_paths: Vec::new(),
            notes: String::new(),
            username: None,
            password: None,
            created_at: current_timestamp(),
            last_connected: None,
        }
    }
}

impl Profile {
    /// 建立新 Profile（測試用）
    #[cfg(test)]
    pub fn new(name: &str, display_name: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            ..Default::default()
        }
    }

    /// 設定連線資訊（測試用）
    #[cfg(test)]
    pub fn with_connection(mut self, host: &str, port: &str) -> Self {
        self.connection = ConnectionConfig {
            host: host.to_string(),
            port: port.to_string(),
        };
        self
    }
}

// ============================================================================
// 九宮格 Numpad 設定
// ============================================================================

/// 九宮格快捷鍵設定（可自訂每個按鍵對應的指令）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumpadConfig {
    /// 是否啟用九宮格行走模式
    #[serde(default)]
    pub enabled: bool,
    /// Numpad 7 指令
    #[serde(default = "default_numpad_7")]
    pub key_7: String,
    /// Numpad 8 指令
    #[serde(default = "default_numpad_8")]
    pub key_8: String,
    /// Numpad 9 指令
    #[serde(default = "default_numpad_9")]
    pub key_9: String,
    /// Numpad 4 指令
    #[serde(default = "default_numpad_4")]
    pub key_4: String,
    /// Numpad 5 指令
    #[serde(default = "default_numpad_5")]
    pub key_5: String,
    /// Numpad 6 指令
    #[serde(default = "default_numpad_6")]
    pub key_6: String,
    /// Numpad 1 指令
    #[serde(default = "default_numpad_1")]
    pub key_1: String,
    /// Numpad 2 指令
    #[serde(default = "default_numpad_2")]
    pub key_2: String,
    /// Numpad 3 指令
    #[serde(default = "default_numpad_3")]
    pub key_3: String,
    /// Numpad 0 指令
    #[serde(default = "default_numpad_0")]
    pub key_0: String,
    /// Numpad . 指令
    #[serde(default = "default_numpad_dot")]
    pub key_dot: String,
}

fn default_numpad_7() -> String { "northwest".to_string() }
fn default_numpad_8() -> String { "north".to_string() }
fn default_numpad_9() -> String { "northeast".to_string() }
fn default_numpad_4() -> String { "west".to_string() }
fn default_numpad_5() -> String { "look".to_string() }
fn default_numpad_6() -> String { "east".to_string() }
fn default_numpad_1() -> String { "southwest".to_string() }
fn default_numpad_2() -> String { "south".to_string() }
fn default_numpad_3() -> String { "southeast".to_string() }
fn default_numpad_0() -> String { "down".to_string() }
fn default_numpad_dot() -> String { "up".to_string() }

impl Default for NumpadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key_7: default_numpad_7(),
            key_8: default_numpad_8(),
            key_9: default_numpad_9(),
            key_4: default_numpad_4(),
            key_5: default_numpad_5(),
            key_6: default_numpad_6(),
            key_1: default_numpad_1(),
            key_2: default_numpad_2(),
            key_3: default_numpad_3(),
            key_0: default_numpad_0(),
            key_dot: default_numpad_dot(),
        }
    }
}

impl NumpadConfig {
    /// 根據數字索引取得對應的指令（0-9 對應數字鍵，10 對應小數點鍵）
    pub fn command_for_index(&self, index: u8) -> Option<&str> {
        let cmd = match index {
            0 => &self.key_0,
            1 => &self.key_1,
            2 => &self.key_2,
            3 => &self.key_3,
            4 => &self.key_4,
            5 => &self.key_5,
            6 => &self.key_6,
            7 => &self.key_7,
            8 => &self.key_8,
            9 => &self.key_9,
            10 => &self.key_dot,
            _ => return None,
        };
        if cmd.is_empty() { None } else { Some(cmd) }
    }
}

// ============================================================================
// 新架構：GlobalConfig
// ============================================================================

/// UI 偏好設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// 視窗寬度
    #[serde(default = "default_window_width")]
    pub window_width: f32,
    /// 視窗高度
    #[serde(default = "default_window_height")]
    pub window_height: f32,
    /// 字型大小
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// 字型家族名稱（空字串 = 內建 Sarasa Mono TC）
    #[serde(default)]
    pub font_family: String,
    /// 深色模式（true = 深色，false = 淺色）
    #[serde(default = "default_true")]
    pub dark_mode: bool,
    /// 自動重連
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    /// 自動重連延遲（秒）
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,
    /// 啟用文字閃爍效果
    #[serde(default = "default_true")]
    pub enable_blink: bool,
}

fn default_window_width() -> f32 {
    1024.0
}
fn default_window_height() -> f32 {
    768.0
}
fn default_font_size() -> f32 {
    14.0
}
fn default_reconnect_delay() -> u64 {
    3
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_width: default_window_width(),
            window_height: default_window_height(),
            font_size: default_font_size(),
            font_family: String::new(),
            dark_mode: true,
            auto_reconnect: true,
            reconnect_delay_secs: default_reconnect_delay(),
            enable_blink: true,
        }
    }
}

/// 全域設定（跨 Profile 共用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 全域別名（所有連線生效）
    #[serde(default)]
    pub global_aliases: Vec<AliasConfig>,
    /// 全域觸發器（所有連線生效）
    #[serde(default)]
    pub global_triggers: Vec<TriggerConfig>,
    /// 啟動時自動連線的 Profile 名稱列表
    #[serde(default)]
    pub auto_connect_profiles: Vec<String>,
    /// UI 設定
    #[serde(default)]
    pub ui: UiConfig,
    /// 九宮格快捷鍵設定
    #[serde(default)]
    pub numpad: NumpadConfig,
    /// 設定檔版本（用於未來遷移）
    #[serde(default = "default_config_version")]
    pub config_version: u32,
}

fn default_config_version() -> u32 {
    2 // 版本 2 = 多 Profile 架構
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            global_aliases: Vec::new(),
            global_triggers: Vec::new(),
            auto_connect_profiles: Vec::new(),
            ui: UiConfig::default(),
            numpad: NumpadConfig::default(),
            config_version: default_config_version(),
        }
    }
}

impl GlobalConfig {
    /// 獲取全域設定檔路徑
    pub fn config_path() -> PathBuf {
        config_dir().join("global_config.json")
    }

    /// 從檔案載入全域設定
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// 儲存全域設定到檔案（原子寫入）
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        ensure_parent_dir(&path)?;
        let content = serde_json::to_string_pretty(self)?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }
}

// ============================================================================
// ProfileManager
// ============================================================================

/// Profile 管理器
#[derive(Debug)]
pub struct ProfileManager {
    /// 所有已載入的 Profile (name -> Profile)
    profiles: HashMap<String, Profile>,
}

impl ProfileManager {
    /// 建立新的 ProfileManager 並載入所有 Profile
    pub fn new() -> Self {
        let mut manager = Self {
            profiles: HashMap::new(),
        };
        manager.load_all();
        manager
    }

    /// 獲取 profiles 目錄路徑
    pub fn profiles_dir() -> PathBuf {
        config_dir().join("profiles")
    }

    /// 載入所有 Profile
    pub fn load_all(&mut self) {
        let dir = Self::profiles_dir();
        if !dir.exists() {
            return;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(profile) = serde_json::from_str::<Profile>(&content) {
                            self.profiles.insert(profile.name.clone(), profile);
                        }
                    }
                }
            }
        }
    }

    /// 取得 Profile 列表
    pub fn list(&self) -> Vec<&Profile> {
        self.profiles.values().collect()
    }

    /// 取得單一 Profile
    pub fn get(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// 取得可變參照
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Profile> {
        self.profiles.get_mut(name)
    }

    /// 新增或更新 Profile
    pub fn save(&mut self, profile: Profile) -> Result<(), std::io::Error> {
        // 防止 path traversal：Profile name 僅允許 ASCII 字母數字、底線、連字號
        if !is_safe_profile_name(&profile.name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("不安全的 profile 名稱: {}", profile.name),
            ));
        }

        let dir = Self::profiles_dir();
        ensure_parent_dir(&dir.join("_"))?;

        let path = dir.join(format!("{}.json", profile.name));
        let content = serde_json::to_string_pretty(&profile)?;
        // 原子寫入：先寫暫存檔再 rename
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;

        self.profiles.insert(profile.name.clone(), profile);
        Ok(())
    }

    /// 刪除 Profile
    pub fn delete(&mut self, name: &str) -> Result<(), std::io::Error> {
        if !is_safe_profile_name(name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("不安全的 profile 名稱: {}", name),
            ));
        }
        let path = Self::profiles_dir().join(format!("{}.json", name));
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.profiles.remove(name);
        Ok(())
    }

    /// 複製 Profile
    pub fn duplicate(&mut self, source_name: &str, new_name: &str) -> Result<(), std::io::Error> {
        if let Some(source) = self.profiles.get(source_name).cloned() {
            let mut new_profile = source;
            new_profile.name = new_name.to_string();
            new_profile.display_name = format!("{} (複製)", new_profile.display_name);
            new_profile.created_at = current_timestamp();
            new_profile.last_connected = None;
            self.save(new_profile)?;
        }
        Ok(())
    }

    /// 檢查 Profile 是否存在
    pub fn exists(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 工具函數
// ============================================================================

/// 獲取設定目錄
pub fn config_dir() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("mudclient")
    } else {
        PathBuf::from(".")
    }
}

/// 確保目錄存在
fn ensure_parent_dir(path: &PathBuf) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// 檢查 profile 名稱是否安全（僅允許 ASCII 字母數字、底線、連字號，1-64 字元）
fn is_safe_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// 取得當前 Unix timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_creation() {
        let profile = Profile::new("test", "測試帳號")
            .with_connection("localhost", "7777");
        
        assert_eq!(profile.name, "test");
        assert_eq!(profile.display_name, "測試帳號");
        assert_eq!(profile.connection.host, "localhost");
        assert_eq!(profile.connection.port, "7777");
    }

    #[test]
    fn test_profile_serialization() {
        let profile = Profile::new("test", "測試");
        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: Profile = serde_json::from_str(&json).unwrap();
        
        assert_eq!(profile.name, deserialized.name);
    }

    #[test]
    fn test_global_config_defaults() {
        let config = GlobalConfig::default();
        
        assert!(config.global_aliases.is_empty());
        assert!(config.global_triggers.is_empty());
        assert!(config.auto_connect_profiles.is_empty());
        assert_eq!(config.config_version, 2);
    }
}
