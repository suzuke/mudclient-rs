//! MUD Core Library
//!
//! 提供 MUD 客戶端的核心功能：
//! - `telnet`: Telnet 協定連線與資料處理
//! - `encoding`: Big5/UTF-8 編解碼
//! - `buffer`: 訊息歷史緩衝區
//! - `alias`: 命令別名系統
//! - `logger`: 日誌記錄
//! - `trigger`: 觸發器系統
//! - `script`: Python 腳本支援
//! - `window`: 多視窗管理

pub mod alias;
pub mod buffer;
pub mod debug_log;
pub mod encoding;
pub mod logger;
pub mod paths;
pub mod script;
pub mod speedwalk;
pub mod telnet;
pub mod trigger;
pub mod window;

pub use alias::{Alias, AliasManager};
pub use buffer::MessageBuffer;
pub use encoding::{decode_big5, encode_big5};
pub use logger::{LogFormat, Logger};
pub use paths::{Path, PathManager};
pub use speedwalk::parse_speedwalk;
pub use script::{MudContext, ScriptEngine};
pub use telnet::TelnetClient;
pub use trigger::{Trigger, TriggerAction, TriggerManager, TriggerPattern};
pub use window::{SubWindow, WindowManager, WindowMessage};
