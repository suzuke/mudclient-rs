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
    /// 寫入計數（用於定期 flush）
    log_count: u32,
    /// 上一條訊息（用於摺疊）
    last_message: Option<String>,
    /// 重複計數
    repeat_count: usize,
}

impl Logger {
    /// 創建新的日誌記錄器
    pub fn new() -> Self {
        Self {
            path: None,
            writer: None,
            format: LogFormat::default(),
            recording: false,
            log_count: 0,
            last_message: None,
            repeat_count: 0,
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

        // 寫入剩餘的 pending 訊息
        self.write_pending_message()?;

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

    /// 寫入當前緩衝的 pending 訊息
    fn write_pending_message(&mut self) -> Result<(), LogError> {
        if let Some(msg) = self.last_message.take() {
            let writer = self.writer.as_mut().ok_or(LogError::NotOpen)?;
            
            let final_msg = if self.repeat_count > 1 {
                format!("{} [x{}]", msg, self.repeat_count)
            } else {
                msg
            };

            match self.format {
                LogFormat::PlainText => {
                    let clean = Self::strip_ansi(&final_msg);
                    writeln!(writer, "{}", clean)?;
                }
                LogFormat::Raw => {
                    writeln!(writer, "{}", final_msg)?;
                }
                LogFormat::Html => {
                    let html = Self::ansi_to_html(&final_msg);
                    writeln!(writer, "{}<br>", html)?;
                }
            }
            self.repeat_count = 0;
        }
        Ok(())
    }

    /// 記錄訊息
    pub fn log(&mut self, message: &str) -> Result<(), LogError> {
        if !self.recording {
            return Ok(()); // 靜默忽略
        }

        // 移除尾部換行，避免 writeln! 產生多餘空行
        let message = message.trim_end_matches(['\n', '\r']);

        // 跳過空白行，避免房間敘述間產生多餘空行
        if message.trim().is_empty() {
            return Ok(());
        }

        // 檢查是否與上一條訊息相同
        if let Some(last) = &self.last_message {
            if last == message {
                self.repeat_count += 1;
                return Ok(());
            }
        }

        // 訊息不同，先寫入上一條
        self.write_pending_message()?;

        // 更新為新訊息
        self.last_message = Some(message.to_string());
        self.repeat_count = 1;

        // 每 50 條訊息自動 flush，避免異常退出時丟失日誌
        self.log_count += 1;
        if self.log_count >= 50 {
            self.log_count = 0;
            self.flush()?;
        }

        Ok(())
    }

    /// 刷新緩衝區
    pub fn flush(&mut self) -> Result<(), LogError> {
        self.write_pending_message()?;
        if let Some(ref mut writer) = self.writer {
            writer.flush()?;
        }
        Ok(())
    }

    /// 移除 ANSI 轉義碼（委託至 crate::util::strip_ansi）
    fn strip_ansi(input: &str) -> String {
        crate::util::strip_ansi(input)
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

/// 壓縮指定目錄中超過 `days` 天的 .txt log 檔為 .txt.gz
pub fn compress_old_logs(log_dir: &str, days: u64) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(days * 24 * 3600);

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension() else { continue };
        if ext != "txt" {
            continue;
        }
        let Ok(meta) = path.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        if modified > cutoff {
            continue;
        }

        // 壓縮
        let gz_path = path.with_extension("txt.gz");
        match compress_file(&path, &gz_path) {
            Ok(()) => {
                let _ = fs::remove_file(&path);
                tracing::info!("compressed log: {}", path.display());
            }
            Err(e) => {
                tracing::warn!("failed to compress {}: {}", path.display(), e);
                let _ = fs::remove_file(&gz_path);
            }
        }
    }
}

fn compress_file(src: &Path, dst: &Path) -> Result<(), LogError> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let input = fs::read(src)?;
    let out_file = File::create(dst)?;
    let mut encoder = GzEncoder::new(out_file, Compression::default());
    encoder.write_all(&input)?;
    encoder.finish()?;
    Ok(())
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

    #[test]
    fn test_log_folding() {
        let temp_dir = std::env::temp_dir();
        let log_path = temp_dir.join("test_mud_log_folding.txt");

        let mut logger = Logger::new();
        logger.set_format(LogFormat::PlainText);
        logger.start(&log_path).unwrap();

        logger.log("Line A").unwrap();
        logger.log("Line B").unwrap();
        logger.log("Line B").unwrap();
        logger.log("Line B").unwrap();
        logger.log("Line C").unwrap();
        
        logger.stop().unwrap();

        let content = fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Line A");
        assert_eq!(lines[1], "Line B [x3]");
        assert_eq!(lines[2], "Line C");

        let _ = fs::remove_file(&log_path);
    }

    #[test]
    fn test_compress_old_logs() {
        use std::time::{Duration, SystemTime};

        let temp_dir = std::env::temp_dir().join("test_compress_logs");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // 建立一個「舊」log 檔
        let old_file = temp_dir.join("old_log.txt");
        fs::write(&old_file, "old log content").unwrap();
        // 將修改時間設為 8 天前
        let eight_days_ago = SystemTime::now() - Duration::from_secs(8 * 24 * 3600);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(eight_days_ago)).unwrap();

        // 建立一個「新」log 檔
        let new_file = temp_dir.join("new_log.txt");
        fs::write(&new_file, "new log content").unwrap();

        // 建立一個已壓縮的檔案（不應被處理）
        let gz_file = temp_dir.join("already.txt.gz");
        fs::write(&gz_file, "fake gz").unwrap();

        // 執行壓縮
        compress_old_logs(temp_dir.to_str().unwrap(), 7);

        // 舊檔案應被壓縮
        assert!(!old_file.exists(), "old .txt should be removed");
        assert!(temp_dir.join("old_log.txt.gz").exists(), ".gz should exist");

        // 新檔案不變
        assert!(new_file.exists(), "new .txt should remain");
        assert!(!temp_dir.join("new_log.txt.gz").exists());

        // 已壓縮檔不變
        assert!(gz_file.exists());

        // 驗證解壓內容正確
        let gz_data = fs::read(temp_dir.join("old_log.txt.gz")).unwrap();
        let mut decoder = flate2::read::GzDecoder::new(&gz_data[..]);
        let mut decoded = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut decoded).unwrap();
        assert_eq!(decoded, "old log content");

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
