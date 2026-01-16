//! Telnet 協定模組
//!
//! 實作 Telnet 連線管理和基本協定處理

mod client;
mod protocol;

pub use client::TelnetClient;
pub use protocol::{TelnetCommand, TelnetOption};
