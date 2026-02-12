//! Telnet 客戶端
//!
//! 非同步 Telnet 連線管理

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::protocol::{generate_refusal, parse_telnet_data, TelnetEvent};
use crate::encoding::encode_big5;

/// Telnet 客戶端錯誤
#[derive(Debug, Error)]
pub enum TelnetError {
    #[error("連線失敗: {0}")]
    ConnectionFailed(#[from] io::Error),

    #[error("連線逾時")]
    Timeout,

    #[error("未連線")]
    NotConnected,

    #[error("DNS 解析失敗: {0}")]
    DnsResolutionFailed(String),
}

/// 連線狀態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

/// Telnet 客戶端配置
#[derive(Debug, Clone)]
pub struct TelnetConfig {
    /// 連線逾時（秒）
    pub connect_timeout: Duration,
    /// 讀取緩衝區大小
    pub read_buffer_size: usize,
}

impl Default for TelnetConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            read_buffer_size: 8192,
        }
    }
}

/// Telnet 客戶端
pub struct TelnetClient {
    stream: Option<TcpStream>,
    config: TelnetConfig,
    state: ConnectionState,
    /// 尚未處理的原始位元組緩衝區（Telnet 協定層）
    raw_buffer: Vec<u8>,
    /// 已處理 Telnet 協定但尚未解碼為 UTF-8 的位元組緩衝區
    text_buffer: Vec<u8>,
    /// 暫存等待 Big5 尾位元組時到達的 ANSI 序列
    pending_ansi: Vec<(String, usize)>,
    /// 暫存尚未完整的 ANSI 序列 (以 ESC \x1b 開頭)
    ansi_buffer: Vec<u8>,
    /// Big5 解碼器（保留用於相容，但已切換為手動狀態機處理）
    _decoder: encoding_rs::Decoder,
}

impl TelnetClient {
    /// 創建新的 Telnet 客戶端
    pub fn new(config: TelnetConfig) -> Self {
        Self {
            stream: None,
            config,
            state: ConnectionState::Disconnected,
            raw_buffer: Vec::new(),
            text_buffer: Vec::new(),
            pending_ansi: Vec::new(),
            ansi_buffer: Vec::new(),
            _decoder: encoding_rs::BIG5.new_decoder(),
        }
    }

    /// 獲取連線狀態
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// 連線到 MUD 伺服器
    ///
    /// # Arguments
    /// * `host` - 主機名稱或 IP
    /// * `port` - 連接埠
    pub async fn connect(&mut self, host: &str, port: u16) -> Result<(), TelnetError> {
        self.state = ConnectionState::Connecting;
        info!("正在連線到 {}:{}", host, port);

        // 解析主機名稱
        let addr = format!("{}:{}", host, port);
        let socket_addrs: Vec<SocketAddr> = tokio::net::lookup_host(&addr)
            .await
            .map_err(|e| TelnetError::DnsResolutionFailed(e.to_string()))?
            .collect();

        if socket_addrs.is_empty() {
            return Err(TelnetError::DnsResolutionFailed(format!(
                "無法解析主機: {}",
                host
            )));
        }

        debug!("已解析到位址: {:?}", socket_addrs);

        // 嘗試連線
        let stream = timeout(
            self.config.connect_timeout,
            TcpStream::connect(&socket_addrs[0]),
        )
        .await
        .map_err(|_| TelnetError::Timeout)?
        .map_err(TelnetError::ConnectionFailed)?;

        // 設定 TCP 選項
        stream.set_nodelay(true)?;

        info!("已連線到 {}:{}", host, port);
        self.stream = Some(stream);
        self.state = ConnectionState::Connected;

        Ok(())
    }

    /// 斷開連線
    pub async fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.into_std(); // 讓 stream 自動關閉
        }
        self.state = ConnectionState::Disconnected;
        info!("已斷開連線");
    }

    /// 發送文字到伺服器（會自動編碼為 Big5 並加上 CRLF）
    pub async fn send(&mut self, text: &str) -> Result<(), TelnetError> {
        let stream = self.stream.as_mut().ok_or(TelnetError::NotConnected)?;

        let mut data = encode_big5(text);
        data.extend_from_slice(b"\r\n");

        stream.write_all(&data).await?;
        stream.flush().await?;

        debug!("已發送: {}", text);
        Ok(())
    }

    /// 發送原始位元組到伺服器
    pub async fn send_raw(&mut self, data: &[u8]) -> Result<(), TelnetError> {
        let stream = self.stream.as_mut().ok_or(TelnetError::NotConnected)?;
        stream.write_all(data).await?;
        stream.flush().await?;
        Ok(())
    }

    /// 讀取資料並處理 Telnet 協定
    ///
    /// 返回 (解碼後的 UTF-8 文字, 每個字元對應的原始位元組寬度)
    pub async fn read_with_widths(&mut self) -> Result<(String, Vec<u8>), TelnetError> {
        let stream = self.stream.as_mut().ok_or(TelnetError::NotConnected)?;

        let mut buffer = vec![0u8; self.config.read_buffer_size];
        let n = stream.read(&mut buffer).await?;

        if n == 0 {
            self.state = ConnectionState::Disconnected;
            return Err(TelnetError::NotConnected);
        }

        self.raw_buffer.extend_from_slice(&buffer[..n]);
        crate::debug_log::DebugLogger::log_bytes("READ_RAW", &buffer[..n]);

        let (text_bytes, events, consumed) = parse_telnet_data(&self.raw_buffer);
        self.raw_buffer.drain(..consumed);

        // 處理 Telnet 事件
        for event in events {
            if let TelnetEvent::Command(cmd, option) = event {
                let response = generate_refusal(cmd, option);
                if !response.is_empty() {
                    let _ = self.send_raw(&response).await;
                }
            }
        }

        let (final_output, final_widths) = self.process_byte_stream(&text_bytes);
        Ok((final_output, final_widths))
    }

    /// 處理位元組流：處理 Big5 解碼與 ANSI 序列
    /// 公開此方法以便測試
    pub fn process_byte_stream(&mut self, text_bytes: &[u8]) -> (String, Vec<u8>) {
        let mut final_output = String::new();
        let mut final_widths = Vec::new();
        let mut i = 0;

        while i < text_bytes.len() {
            let b = text_bytes[i];

            // 1. 處理 ANSI 緩衝區（如果在消費中）
            if !self.ansi_buffer.is_empty() {
                self.ansi_buffer.push(b);
                i += 1;
                
                let is_complete = if self.ansi_buffer.len() > 1 && self.ansi_buffer[1] == b'[' {
                    // CSI 序列 (ESC [ ...): 必須至少 3 字元，且最後一個是 0x40-0x7E
                    self.ansi_buffer.len() > 2 && (0x40..=0x7E).contains(&b)
                } else if self.ansi_buffer.len() >= 2 {
                    // 一般轉義序列 (ESC x): 2 字元即結束
                    true
                } else {
                    false
                };

                if is_complete {
                    if let Ok(ansi_str) = std::str::from_utf8(&self.ansi_buffer) {
                        let count = ansi_str.chars().count();
                        if !self.text_buffer.is_empty() {
                            // 夾在 Big5 字元中間的 ANSI，暫存
                            self.pending_ansi.push((ansi_str.to_string(), count));
                        } else {
                            // 正常的 ANSI，直接輸出
                            for ch in ansi_str.chars() {
                                final_output.push(ch);
                                final_widths.push(0);
                            }
                        }
                    }
                    self.ansi_buffer.clear();
                }
                continue;
            }

            // 2. 偵測 ANSI 開始 (ESC)
            if b == 0x1B {
                self.ansi_buffer.push(b);
                i += 1;
                continue;
            }

            // 3. 數據位元組：進入 Big5 重組流程
            self.text_buffer.push(b);
            i += 1;

            let first = self.text_buffer[0];
            // Big5 定義：Leading 0x81-0xFE, Trailing 0x40-0x7E, 0xA1-0xFE
            // 簡化判斷：如果是先導位元組且緩衝區還只有 1 字元，則等待
            let is_complete = if first < 0x81 || first == 0xFF {
                true // ASCII 或其他特殊位元組
            } else {
                self.text_buffer.len() >= 2
            };

            if is_complete {
                // 解碼目前緩衝區中的 1-2 位元組
                // 使用 stateless 解碼避免 decoder 狀態不一致問題
                use encoding_rs::BIG5;
                let (res, _read, _replaced) = BIG5.decode(&self.text_buffer);
                let ch_str = res.to_string();

                // 啟發式 ANSI 放置法則
                // [m (Bare Reset) 通常用於雙色字技巧，必須放在字元前
                let has_bare_reset = self.pending_ansi.iter().any(|(s, _)| s == "\x1b[m");

                if has_bare_reset {
                    // [m 在前模式：適合雙色字
                    for (s, _) in self.pending_ansi.drain(..) {
                        for ch in s.chars() {
                            final_output.push(ch);
                            final_widths.push(0);
                        }
                    }
                    for ch in ch_str.chars() {
                        let w = if ch.is_ascii() { 1 } else { 2 };
                        final_output.push(ch);
                        final_widths.push(w);
                    }
                } else {
                    // 常規模式：字元在前（小紅帽、Boots 固定顏色）
                    for ch in ch_str.chars() {
                        let w = if ch.is_ascii() { 1 } else { 2 };
                        final_output.push(ch);
                        final_widths.push(w);
                    }
                    for (s, _) in self.pending_ansi.drain(..) {
                        for ch in s.chars() {
                            final_output.push(ch);
                            final_widths.push(0);
                        }
                    }
                }
                self.text_buffer.clear();
            }
        }

        // 4. 標點符號正規化：將半形逗號統一為全形
        let mut normalized_output = String::with_capacity(final_output.len());
        let mut normalized_widths = Vec::with_capacity(final_widths.len());
        let chars: Vec<char> = final_output.chars().collect();
        let mut j = 0;
        while j < chars.len() {
            let ch = chars[j];
            let w = final_widths[j];
            
            // 偵測 ", " (逗號 + 空格) -> "，"
            if ch == ',' && w == 1 && j + 1 < chars.len() && chars[j+1] == ' ' && final_widths[j+1] == 1 {
                normalized_output.push('，');
                normalized_widths.push(2);
                j += 2;
                continue;
            }
            
            // 偵測單個 "," -> "，"
            if ch == ',' && w == 1 {
                normalized_output.push('，');
                normalized_widths.push(2);
                j += 1;
                continue;
            }

            // 偵測 ". " (句點 + 空格) -> "。"
            if ch == '.' && w == 1 && j + 1 < chars.len() && chars[j+1] == ' ' && final_widths[j+1] == 1 {
                normalized_output.push('。');
                normalized_widths.push(2);
                j += 2;
                continue;
            }

            // 偵測單個 "." -> "。"
            // 排除：1. 刪節號 (..) 2. 數字中點 (1.5)
            if ch == '.' && w == 1 {
                let prev_is_dot = j > 0 && chars[j-1] == '.' && final_widths[j-1] == 1;
                let next_is_dot = j + 1 < chars.len() && chars[j+1] == '.' && final_widths[j+1] == 1;
                let prev_is_digit = j > 0 && chars[j-1].is_ascii_digit();
                let next_is_digit = j + 1 < chars.len() && chars[j+1].is_ascii_digit();

                if !prev_is_dot && !next_is_dot && !(prev_is_digit && next_is_digit) {
                    normalized_output.push('。');
                    normalized_widths.push(2);
                    j += 1;
                    continue;
                }
            }
            
            normalized_output.push(ch);
            normalized_widths.push(w);
            j += 1;
        }

        (normalized_output, normalized_widths)
    }

    /// 向後相容的 read
    pub async fn read(&mut self) -> Result<String, TelnetError> {
        self.read_with_widths().await.map(|(s, _)| s)
    }

    /// 啟動非同步讀取迴圈，將接收到的資料發送到 channel
    pub async fn start_read_loop(
        mut self,
        tx: mpsc::Sender<String>,
        mut shutdown: mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                result = self.read() => {
                    match result {
                        Ok(text) => {
                            if !text.is_empty() {
                                if tx.send(text).await.is_err() {
                                    warn!("接收端已關閉");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("讀取錯誤: {}", e);
                            break;
                        }
                    }
                }
                _ = shutdown.recv() => {
                    info!("收到關閉信號");
                    break;
                }
            }
        }

        self.disconnect().await;
    }
}

impl Default for TelnetClient {
    fn default() -> Self {
        Self::new(TelnetConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TelnetClient::default();
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_config_default() {
        let config = TelnetConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.read_buffer_size, 8192);
    }

    #[tokio::test]
    async fn test_send_without_connection() {
        let mut client = TelnetClient::default();
        let result = client.send("test").await;
        assert!(matches!(result, Err(TelnetError::NotConnected)));
    }

    #[tokio::test]
    async fn test_read_without_connection() {
        let mut client = TelnetClient::default();
        let result = client.read().await;
        assert!(matches!(result, Err(TelnetError::NotConnected)));
    }

    #[test]
    fn test_big5_split_with_ansi_across_calls() {
        // 模擬 "泉" 分兩次送達，且中間夾帶 ANSI
        let mut client = TelnetClient::default();
        let input1 = vec![0xAC];
        let (out1, _) = client.process_byte_stream(&input1);
        assert_eq!(out1, "");
        
        // 第二包：ANSI + 0x75 ([0m explicit reset)
        let input2 = vec![0x1B, 0x5B, 0x30, 0x6D, 0x75];
        let (out2, _) = client.process_byte_stream(&input2);
        
        // [0m 應該放在字元後
        assert_eq!(out2, "泉\x1b[0m");
    }

    #[test]
    fn test_big5_split_by_ansi() {
         // 模擬 "泉" (Big5: 0xAC 0x75) 被 ANSI \x1b[0m 打斷 (一次性輸入)
         // Input sequence: 0xAC, \x1b, [, 0, m, 0x75
         let mut client = TelnetClient::default();
         
         let input = vec![0xAC, 0x1B, 0x5B, 0x30, 0x6D, 0x75];
         let (output, _) = client.process_byte_stream(&input);
         
         // [0m 應該放在字元後
         assert_eq!(output, "泉\x1b[0m");
    }

    #[test]
    fn test_big5_split_with_bare_reset() {
        // 模擬 "蠻" (假設 split) 中間夾帶 [m (Bare Reset)
        let mut client = TelnetClient::default();
        let input1 = vec![0xAC]; // 借用 AC (泉) 來測試 split 邏輯，雖蠻不是 AC，但 split 邏輯通用
        let (out1, _) = client.process_byte_stream(&input1);
        assert_eq!(out1, "");
        
        // 第二包：[m + 0x75
        let input2 = vec![0x1B, 0x5B, 0x6D, 0x75];
        let (out2, _) = client.process_byte_stream(&input2);
        
        // [m (Bare Reset) 應該放在字元前，以觸發雙色字技巧 (如蠻荒之刃)
        assert_eq!(out2, "\x1b[m泉");
    }

    #[test]
    fn test_punctuation_normalization() {
        let mut client = TelnetClient::default();
        
        // 1. 測試逗號
        let (out1, _) = client.process_byte_stream(b"A, B, C");
        assert_eq!(out1, "A，B，C");
        
        client = TelnetClient::default();
        // 2. 測試句點
        let (out2, _) = client.process_byte_stream(b"City Center. Next to the park.");
        assert_eq!(out2, "City Center。Next to the park。");
        
        client = TelnetClient::default();
        // 3. 測試句點 + 空格 (". ")
        let (out3, _) = client.process_byte_stream(b"Welcome. Have fun.");
        assert_eq!(out3, "Welcome。Have fun。");

        client = TelnetClient::default();
        // 4. 測試例外：數字 (不應轉換)
        let (out4, _) = client.process_byte_stream(b"Level 1.5, version 2.0");
        assert_eq!(out4, "Level 1.5，version 2.0"); // 逗號轉換了，點沒變

        client = TelnetClient::default();
        // 5. 測試例外：刪節號 (不應轉換)
        let (out5, _) = client.process_byte_stream(b"Loading...");
        assert_eq!(out5, "Loading...");
        
        client = TelnetClient::default();
        // 6. 測試 ANSI 夾雜
        let (out6, _) = client.process_byte_stream(b"\x1b[31mHot, \x1b[34mCold.\x1b[m");
        assert_eq!(out6, "\x1b[31mHot，\x1b[34mCold。\x1b[m");
    }

}
