//! ANSI 轉義碼解析模組
//!
//! 解析 MUD 伺服器發送的 ANSI 顏色碼

use eframe::egui::Color32;

/// ANSI 顏色解析後的文字片段
#[derive(Debug, Clone)]
pub struct AnsiSpan {
    pub text: String,
    pub fg_color: Color32,
}

impl Default for AnsiSpan {
    fn default() -> Self {
        Self {
            text: String::new(),
            fg_color: Color32::from_rgb(200, 200, 200),
        }
    }
}

/// ANSI 解析器狀態
#[derive(Default, Clone)]
struct AnsiState {
    fg_color: Color32,
    bold: bool,
}

impl AnsiState {
    fn new() -> Self {
        Self {
            fg_color: Color32::from_rgb(200, 200, 200),
            bold: false,
        }
    }

    fn reset(&mut self) {
        self.fg_color = Color32::from_rgb(200, 200, 200);
        self.bold = false;
    }

    /// 獲取當前顏色（考慮 bold 加亮）
    fn current_color(&self) -> Color32 {
        self.fg_color
    }

    fn apply_code(&mut self, code: u8) {
        match code {
            0 => self.reset(),
            1 => {
                self.bold = true;
                // 如果已經有顏色，加亮它
                self.brighten_current_color();
            }
            2 | 22 => self.bold = false,
            // 閃爍、反轉等效果忽略（5, 7, 8, 27 等）
            5 | 6 | 7 | 8 | 25 | 27 | 28 => {}
            // 前景色
            30 => self.set_fg(0, 0, 0),
            31 => self.set_fg(187, 0, 0),
            32 => self.set_fg(0, 187, 0),
            33 => self.set_fg(187, 187, 0),
            34 => self.set_fg(0, 0, 187),
            35 => self.set_fg(187, 0, 187),
            36 => self.set_fg(0, 187, 187),
            37 => self.set_fg(187, 187, 187),
            39 => self.fg_color = Color32::from_rgb(200, 200, 200), // 默認前景色
            // 高亮前景色 (90-97)
            90 => self.set_fg(128, 128, 128),
            91 => self.set_fg(255, 85, 85),
            92 => self.set_fg(85, 255, 85),
            93 => self.set_fg(255, 255, 85),
            94 => self.set_fg(85, 85, 255),
            95 => self.set_fg(255, 85, 255),
            96 => self.set_fg(85, 255, 255),
            97 => self.set_fg(255, 255, 255),
            // 背景色 (40-47, 100-107) - 暫時忽略
            40..=49 | 100..=107 => {}
            _ => {}
        }
    }

    fn set_fg(&mut self, r: u8, g: u8, b: u8) {
        if self.bold {
            // 加亮顏色
            self.fg_color = Color32::from_rgb(
                r.saturating_add(68),
                g.saturating_add(68),
                b.saturating_add(68),
            );
        } else {
            self.fg_color = Color32::from_rgb(r, g, b);
        }
    }

    fn brighten_current_color(&mut self) {
        let [r, g, b, _] = self.fg_color.to_array();
        self.fg_color = Color32::from_rgb(
            r.saturating_add(68),
            g.saturating_add(68),
            b.saturating_add(68),
        );
    }
}

/// 解析 ANSI 轉義碼，返回帶顏色的文字片段
pub fn parse_ansi(input: &str) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let mut state = AnsiState::new();
    let mut current_span = AnsiSpan {
        text: String::new(),
        fg_color: state.current_color(),
    };

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC 開始
            if chars.peek() == Some(&'[') {
                chars.next(); // 消耗 '['

                // 讀取參數（數字和分號）
                let mut params = Vec::new();
                let mut current_param = String::new();

                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() {
                        current_param.push(chars.next().unwrap());
                    } else if ch == ';' {
                        params.push(current_param.parse::<u8>().unwrap_or(0));
                        current_param.clear();
                        chars.next();
                    } else {
                        break;
                    }
                }

                // 最後一個參數
                if !current_param.is_empty() {
                    params.push(current_param.parse::<u8>().unwrap_or(0));
                }

                // 讀取命令字元
                if let Some(cmd) = chars.next() {
                    if cmd == 'm' {
                        // SGR (Select Graphic Rendition)
                        // 保存當前 span（如果有內容）
                        if !current_span.text.is_empty() {
                            spans.push(current_span);
                        }

                        // 解析顏色碼
                        if params.is_empty() {
                            state.reset();
                        } else {
                            for code in params {
                                state.apply_code(code);
                            }
                        }

                        // 開始新 span
                        current_span = AnsiSpan {
                            text: String::new(),
                            fg_color: state.current_color(),
                        };
                    }
                    // 其他 CSI 命令（A, B, C, D, H, J, K 等）直接忽略
                }
            }
            // 其他 ESC 序列也忽略
        } else if c >= ' ' || c == '\n' || c == '\r' || c == '\t' {
            // 可列印字符和控制字符（換行、回車、Tab）
            current_span.text.push(c);
        }
        // 其他控制字符忽略
    }

    // 添加最後一個 span
    if !current_span.text.is_empty() {
        spans.push(current_span);
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let spans = parse_ansi("Hello World");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello World");
    }

    #[test]
    fn test_parse_colored_text() {
        let spans = parse_ansi("\x1b[31mRed\x1b[0m Normal");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Red");
        assert_eq!(spans[1].text, " Normal");
    }

    #[test]
    fn test_parse_bold_color() {
        let spans = parse_ansi("\x1b[1;33mBold Yellow\x1b[0m");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Bold Yellow");
    }

    #[test]
    fn test_parse_blink_ignored() {
        // 閃爍 (5m) 應該被忽略，不影響文字
        let spans = parse_ansi("\x1b[5mBlink\x1b[0m");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Blink");
    }

    #[test]
    fn test_cursor_control_ignored() {
        // 游標控制序列應該被忽略
        let spans = parse_ansi("Hello\x1b[2J\x1b[HWorld");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "HelloWorld");
    }

    #[test]
    fn test_high_intensity_colors() {
        let spans = parse_ansi("\x1b[91mBright Red\x1b[0m");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Bright Red");
        assert_eq!(spans[0].fg_color, Color32::from_rgb(255, 85, 85));
    }
}
