//! 設定檔持久化模組
//!
//! 支援多 Profile 與全域設定的架構：
//! - `GlobalConfig`: 全域設定（全域別名/觸發器、UI 偏好、自動連線列表）
//! - `Profile`: 單一帳號/伺服器的設定（連線資訊、專屬別名/觸發器）
//! - `ProfileManager`: Profile 的 CRUD 操作

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
    /// 自訂腳本載入路徑（可選）
    #[serde(default)]
    pub script_paths: Vec<String>,
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
            script_paths: Vec::new(),
            created_at: current_timestamp(),
            last_connected: None,
        }
    }
}

#[allow(dead_code)]
impl Profile {
    /// 建立新 Profile
    pub fn new(name: &str, display_name: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            ..Default::default()
        }
    }

    /// 設定連線資訊
    pub fn with_connection(mut self, host: &str, port: &str) -> Self {
        self.connection = ConnectionConfig {
            host: host.to_string(),
            port: port.to_string(),
        };
        self
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
    /// 自動重連
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
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

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_width: default_window_width(),
            window_height: default_window_height(),
            font_size: default_font_size(),
            auto_reconnect: true,
        }
    }
}

/// 全域設定（跨 Profile 共用）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// 設定檔版本（用於未來遷移）
    #[serde(default = "default_config_version")]
    pub config_version: u32,
}

fn default_config_version() -> u32 {
    2 // 版本 2 = 多 Profile 架構
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

    /// 儲存全域設定到檔案
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        ensure_parent_dir(&path)?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
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

#[allow(dead_code)]
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

    /// 取得 Profile 名稱列表（排序）
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.profiles.keys().cloned().collect();
        names.sort();
        names
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
        let dir = Self::profiles_dir();
        ensure_parent_dir(&dir.join("_"))?;

        let path = dir.join(format!("{}.json", profile.name));
        let content = serde_json::to_string_pretty(&profile)?;
        fs::write(&path, content)?;

        self.profiles.insert(profile.name.clone(), profile);
        Ok(())
    }

    /// 刪除 Profile
    pub fn delete(&mut self, name: &str) -> Result<(), std::io::Error> {
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

    /// Profile 數量
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// 是否為空
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 舊版相容：AppConfig（保留以支援遷移）
// ============================================================================

/// 舊版應用程式設定（用於遷移）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct LegacyAppConfig {
    pub connection: ConnectionConfig,
    pub aliases: Vec<AliasConfig>,
    pub triggers: Vec<TriggerConfig>,
}

/// 向後相容的類型別名（app.rs 目前仍使用此結構）
/// TODO: 在 Session 系統完成後移除
#[allow(dead_code)]
pub type AppConfig = LegacyAppConfig;

#[allow(dead_code)]
impl LegacyAppConfig {
    /// 設定檔路徑（與舊版相容）
    pub fn config_path() -> PathBuf {
        config_dir().join("config.json")
    }

    /// 舊版設定檔路徑（別名）
    pub fn legacy_config_path() -> PathBuf {
        Self::config_path()
    }

    /// 檢查是否存在舊版設定
    pub fn exists() -> bool {
        Self::legacy_config_path().exists()
    }

    /// 載入設定（與舊版 AppConfig::load() 相容）
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

    /// 載入設定（回傳 Option）
    pub fn try_load() -> Option<Self> {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return Some(config);
                }
            }
        }
        None
    }

    /// 儲存設定到檔案（與舊版 AppConfig::save() 相容）
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        ensure_parent_dir(&path)?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

// ============================================================================
// 遷移邏輯
// ============================================================================

/// 遷移結果
#[derive(Debug)]
#[allow(dead_code)]
pub struct MigrationResult {
    pub migrated: bool,
    pub profile_name: Option<String>,
    pub backup_path: Option<PathBuf>,
}

/// 執行設定遷移
#[allow(dead_code)]
pub fn migrate_legacy_config() -> MigrationResult {
    // 檢查是否需要遷移
    if !LegacyAppConfig::exists() {
        return MigrationResult {
            migrated: false,
            profile_name: None,
            backup_path: None,
        };
    }

    // 檢查是否已經遷移過
    let global_config_path = GlobalConfig::config_path();
    if global_config_path.exists() {
        return MigrationResult {
            migrated: false,
            profile_name: None,
            backup_path: None,
        };
    }

    tracing::info!("偵測到舊版設定檔，開始遷移...");

    // 載入舊版設定
    let legacy = match LegacyAppConfig::try_load() {
        Some(c) => c,
        None => {
            return MigrationResult {
                migrated: false,
                profile_name: None,
                backup_path: None,
            }
        }
    };

    // 備份舊版設定
    let legacy_path = LegacyAppConfig::legacy_config_path();
    let backup_path = config_dir().join("config.json.backup");
    if let Err(e) = fs::copy(&legacy_path, &backup_path) {
        tracing::warn!("備份舊版設定失敗: {}", e);
    }

    // 建立新的全域設定
    let global_config = GlobalConfig {
        auto_connect_profiles: vec!["default".to_string()],
        ..Default::default()
    };
    if let Err(e) = global_config.save() {
        tracing::error!("儲存全域設定失敗: {}", e);
    }

    // 將舊版設定轉換為 Profile
    let profile = Profile {
        name: "default".to_string(),
        display_name: "預設".to_string(),
        connection: legacy.connection,
        aliases: legacy.aliases,
        triggers: legacy.triggers,
        script_paths: Vec::new(),
        created_at: current_timestamp(),
        last_connected: None,
    };

    let mut manager = ProfileManager::new();
    if let Err(e) = manager.save(profile) {
        tracing::error!("儲存 Profile 失敗: {}", e);
    }

    tracing::info!("設定遷移完成！舊版設定已備份至 {:?}", backup_path);

    MigrationResult {
        migrated: true,
        profile_name: Some("default".to_string()),
        backup_path: Some(backup_path),
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
