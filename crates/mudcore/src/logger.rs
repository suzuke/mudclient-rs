//! 日誌記錄模組
//!
//! 自動記錄 MUD 對話到檔案

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 日誌記錄錯誤
#[derive(Debug, Error)]
pub enum LogError {
    #[error("IO 錯誤: {0}")]
    Io(#[from] io::Error),
    
    #[error("日誌未開啟")]
    NotOpen,
}

/// 日誌格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// 純文字（移除 ANSI 顏色碼）
    #[default]
    PlainText,
    /// 原始格式（保留 ANSI 顏色碼）
    Raw,
    /// HTML（將 ANSI 轉換為 HTML 樣式）
    Html,
}

/// 日誌記錄器
pub struct Logger {
    /// 日誌檔案路徑
    path: Option<PathBuf>,
    /// 緩衝寫入器
    writer: Option<BufWriter<File>>,
    /// 日誌格式
    format: LogFormat,
    /// 是否正在記錄
    recording: bool,
}

impl Logger {
    /// 創建新的日誌記錄器
    pub fn new() -> Self {
        Self {
            path: None,
            writer: None,
            format: LogFormat::default(),
            recording: false,
        }
    }

    /// 設置日誌格式
    pub fn set_format(&mut self, format: LogFormat) {
        self.format = format;
    }

    /// 獲取日誌格式
    pub fn format(&self) -> LogFormat {
        self.format
    }

    /// 是否正在記錄
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// 獲取日誌檔案路徑
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 開始記錄到指定檔案
    pub fn start(&mut self, path: impl AsRef<Path>) -> Result<(), LogError> {
        let path = path.as_ref();
        
        // 確保目錄存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        self.writer = Some(BufWriter::new(file));
        self.path = Some(path.to_path_buf());
        self.recording = true;

        // 寫入 HTML 頭部（如果需要）
        if self.format == LogFormat::Html {
            self.write_html_header()?;
        }

        Ok(())
    }

    /// 停止記錄
    pub fn stop(&mut self) -> Result<(), LogError> {
        if !self.recording {
            return Ok(());
        }

        // 寫入 HTML 尾部（如果需要）
        if self.format == LogFormat::Html {
            self.write_html_footer()?;
        }

        // 刷新並關閉檔案
        if let Some(ref mut writer) = self.writer {
            writer.flush()?;
        }

        self.writer = None;
        self.recording = false;
        
        Ok(())
    }

    /// 記錄訊息
    pub fn log(&mut self, message: &str) -> Result<(), LogError> {
        if !self.recording {
            return Ok(()); // 靜默忽略
        }

        let writer = self.writer.as_mut().ok_or(LogError::NotOpen)?;

        match self.format {
            LogFormat::PlainText => {
                let clean = Self::strip_ansi(message);
                writeln!(writer, "{}", clean)?;
            }
            LogFormat::Raw => {
                writeln!(writer, "{}", message)?;
            }
            LogFormat::Html => {
                let html = Self::ansi_to_html(message);
                writeln!(writer, "{}<br>", html)?;
            }
        }

        Ok(())
    }

    /// 刷新緩衝區
    pub fn flush(&mut self) -> Result<(), LogError> {
        if let Some(ref mut writer) = self.writer {
            writer.flush()?;
        }
        Ok(())
    }

    /// 移除 ANSI 轉義碼
    pub fn strip_ansi(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // 跳過 CSI 序列
                if chars.peek() == Some(&'[') {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// 將 ANSI 轉換為 HTML
    fn ansi_to_html(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 2);
        let mut in_span = false;
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    
                    // 讀取參數
                    let mut params = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() || ch == ';' {
                            params.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    // 讀取命令
                    if let Some(cmd) = chars.next() {
                        if cmd == 'm' {
                            // 關閉之前的 span
                            if in_span {
                                result.push_str("</span>");
                                in_span = false;
                            }

                            // 解析顏色
                            if let Some(color) = Self::parse_ansi_color(&params) {
                                result.push_str(&format!(r#"<span style="color: {}">"#, color));
                                in_span = true;
                            }
                        }
                    }
                }
            } else {
                // HTML 轉義
                match c {
                    '<' => result.push_str("&lt;"),
                    '>' => result.push_str("&gt;"),
                    '&' => result.push_str("&amp;"),
                    '\n' => result.push_str("<br>"),
                    _ => result.push(c),
                }
            }
        }

        if in_span {
            result.push_str("</span>");
        }

        result
    }

    /// 解析 ANSI 顏色碼為 CSS 顏色
    fn parse_ansi_color(params: &str) -> Option<&'static str> {
        let codes: Vec<u8> = params
            .split(';')
            .filter_map(|s| s.parse().ok())
            .collect();

        for code in codes {
            match code {
                0 => return None, // 重置
                30 => return Some("#000000"),
                31 => return Some("#bb0000"),
                32 => return Some("#00bb00"),
                33 => return Some("#bbbb00"),
                34 => return Some("#0000bb"),
                35 => return Some("#bb00bb"),
                36 => return Some("#00bbbb"),
                37 => return Some("#bbbbbb"),
                90 => return Some("#808080"),
                91 => return Some("#ff5555"),
                92 => return Some("#55ff55"),
                93 => return Some("#ffff55"),
                94 => return Some("#5555ff"),
                95 => return Some("#ff55ff"),
                96 => return Some("#55ffff"),
                97 => return Some("#ffffff"),
                _ => {}
            }
        }

        None
    }

    /// 寫入 HTML 頭部
    fn write_html_header(&mut self) -> Result<(), LogError> {
        let writer = self.writer.as_mut().ok_or(LogError::NotOpen)?;
        writeln!(writer, r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>MUD Log</title>
<style>
body {{ background: #1e1e1e; color: #d4d4d4; font-family: monospace; white-space: pre-wrap; }}
</style>
</head>
<body>"#)?;
        Ok(())
    }

    /// 寫入 HTML 尾部
    fn write_html_footer(&mut self) -> Result<(), LogError> {
        let writer = self.writer.as_mut().ok_or(LogError::NotOpen)?;
        writeln!(writer, "</body></html>")?;
        Ok(())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[31mRed\x1b[0m Normal";
        let output = Logger::strip_ansi(input);
        assert_eq!(output, "Red Normal");
    }

    #[test]
    fn test_ansi_to_html() {
        let input = "\x1b[31mRed\x1b[0m";
        let output = Logger::ansi_to_html(input);
        assert!(output.contains("color: #bb0000"));
        assert!(output.contains("Red"));
    }

    #[test]
    fn test_logger_lifecycle() {
        let temp_dir = std::env::temp_dir();
        let log_path = temp_dir.join("test_mud_log.txt");

        let mut logger = Logger::new();
        logger.set_format(LogFormat::PlainText);
        
        assert!(!logger.is_recording());
        
        logger.start(&log_path).unwrap();
        assert!(logger.is_recording());
        
        logger.log("Hello World").unwrap();
        logger.log("\x1b[31mColored\x1b[0m").unwrap();
        
        logger.stop().unwrap();
        assert!(!logger.is_recording());

        // 驗證檔案內容
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Hello World"));
        assert!(content.contains("Colored"));
        assert!(!content.contains("\x1b")); // ANSI 已被移除

        // 清理
        let _ = fs::remove_file(&log_path);
    }
}
