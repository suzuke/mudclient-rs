//! MUD Core Library
//!
//! 提供 MUD 客戶端的核心功能：
//! - `telnet`: Telnet 協定連線與資料處理
//! - `encoding`: Big5/UTF-8 編解碼
//! - `buffer`: 訊息歷史緩衝區

pub mod buffer;
pub mod encoding;
pub mod telnet;

pub use buffer::MessageBuffer;
pub use encoding::{decode_big5, encode_big5};
pub use telnet::TelnetClient;
