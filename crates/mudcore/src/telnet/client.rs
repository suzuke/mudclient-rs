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
use crate::encoding::{decode_big5, encode_big5};

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
}

impl TelnetClient {
    /// 創建新的 Telnet 客戶端
    pub fn new(config: TelnetConfig) -> Self {
        Self {
            stream: None,
            config,
            state: ConnectionState::Disconnected,
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
    /// 返回解碼後的 UTF-8 文字
    pub async fn read(&mut self) -> Result<String, TelnetError> {
        let stream = self.stream.as_mut().ok_or(TelnetError::NotConnected)?;

        let mut buffer = vec![0u8; self.config.read_buffer_size];
        let n = stream.read(&mut buffer).await?;

        if n == 0 {
            self.state = ConnectionState::Disconnected;
            return Ok(String::new());
        }

        let raw_data = &buffer[..n];

        // 解析 Telnet 協定
        let (text_data, events) = parse_telnet_data(raw_data);

        // 處理 Telnet 命令（自動回應）
        for event in events {
            match event {
                TelnetEvent::Command(cmd, option) => {
                    debug!("收到 Telnet 命令: {:?} {:?}", cmd, option);
                    let response = generate_refusal(cmd, option);
                    if !response.is_empty() {
                        self.send_raw(&response).await?;
                        debug!("已回應 Telnet 命令");
                    }
                }
                TelnetEvent::Subnegotiation(option, data) => {
                    debug!("收到 Sub-negotiation: {:?}, {} bytes", option, data.len());
                }
                TelnetEvent::Data(_) => {}
            }
        }

        // 將 Big5 解碼為 UTF-8
        let text = decode_big5(&text_data);
        Ok(text)
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
                        Ok(text) if !text.is_empty() => {
                            if tx.send(text).await.is_err() {
                                warn!("接收端已關閉");
                                break;
                            }
                        }
                        Ok(_) => {
                            // 空字串表示連線已關閉
                            info!("伺服器已關閉連線");
                            break;
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
}
