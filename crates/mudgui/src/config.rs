//! 設定檔持久化模組

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 別名設定（可序列化版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasConfig {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
}

/// 觸發器設定（可序列化版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub name: String,
    pub pattern: String,
    pub action: String,
    #[serde(default)]
    pub is_script: bool,
    pub enabled: bool,
}

/// 連線設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: String,
}

/// 完整的應用程式設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub connection: ConnectionConfig,
    pub aliases: Vec<AliasConfig>,
    pub triggers: Vec<TriggerConfig>,
}

impl AppConfig {
    /// 獲取設定檔路徑
    pub fn config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("mudclient").join("config.json")
        } else {
            PathBuf::from("mudclient_config.json")
        }
    }

    /// 從檔案載入設定
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

    /// 儲存設定到檔案
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        
        // 確保目錄存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
