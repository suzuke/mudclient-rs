//! ANSI 轉義碼解析模組
//!
//! 解析 MUD 伺服器發送的 ANSI 顏色碼

use eframe::egui::Color32;

/// ANSI 顏色解析後的文字片段
#[derive(Debug, Clone)]
pub struct AnsiSpan {
    pub text: String,
    pub fg_color: Color32,
    pub bg_color: Option<Color32>,
    pub blink: bool,
}

impl Default for AnsiSpan {
    fn default() -> Self {
        Self {
            text: String::new(),
            fg_color: Color32::from_rgb(200, 200, 200),
            bg_color: None,
            blink: false,
        }
    }
}

/// ANSI 解析器狀態
#[derive(Default, Clone)]
struct AnsiState {
    fg_color: Color32,
    bg_color: Option<Color32>,
    bold: bool,
    blink: bool,
}

impl AnsiState {
    fn new() -> Self {
        Self {
            fg_color: Color32::from_rgb(200, 200, 200),
            bg_color: None,
            bold: false,
            blink: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn current_fg(&self) -> Color32 {
        self.fg_color
    }

    fn current_bg(&self) -> Option<Color32> {
        self.bg_color
    }

    fn apply_code(&mut self, codes: &[u16]) {
        let mut i = 0;
        while i < codes.len() {
            let code = codes[i];
            match code {
                0 => self.reset(),
                1 => {
                    self.bold = true;
                    self.brighten_current_color();
                }
                2 | 22 => self.bold = false,
                5 => self.blink = true,
                25 => self.blink = false,
                // 前景色
                30..=37 => self.fg_color = self.get_basic_color((code - 30) as u8, self.bold),
                38 => {
                    // Extended foreground
                    if i + 2 < codes.len() && codes[i+1] == 5 {
                        self.fg_color = self.get_256_color(codes[i+2] as u8);
                        i += 2;
                    } else if i + 4 < codes.len() && codes[i+1] == 2 {
                        self.fg_color = Color32::from_rgb(codes[i+2] as u8, codes[i+3] as u8, codes[i+4] as u8);
                        i += 4;
                    }
                }
                39 => self.fg_color = Color32::from_rgb(200, 200, 200),
                // 背景色
                40..=47 => self.bg_color = Some(self.get_basic_color((code - 40) as u8, false)),
                48 => {
                    // Extended background
                    if i + 2 < codes.len() && codes[i+1] == 5 {
                        self.bg_color = Some(self.get_256_color(codes[i+2] as u8));
                        i += 2;
                    } else if i + 4 < codes.len() && codes[i+1] == 2 {
                        self.bg_color = Some(Color32::from_rgb(codes[i+2] as u8, codes[i+3] as u8, codes[i+4] as u8));
                        i += 4;
                    }
                }
                49 => self.bg_color = None,
                // 高亮前景色 (90-97)
                90..=97 => self.fg_color = self.get_basic_color((code - 90) as u8, true),
                // 高亮背景色 (100-107)
                100..=107 => self.bg_color = Some(self.get_basic_color((code - 100) as u8, true)),
                _ => {}
            }
            i += 1;
        }
    }

    fn get_basic_color(&self, index: u8, bright: bool) -> Color32 {
        match (index, bright) {
            (0, false) => Color32::from_rgb(0, 0, 0),         // Black
            (0, true)  => Color32::from_rgb(128, 128, 128),   // Gray
            (1, false) => Color32::from_rgb(187, 0, 0),       // Red
            (1, true)  => Color32::from_rgb(255, 85, 85),     // Bright Red
            (2, false) => Color32::from_rgb(0, 187, 0),       // Green
            (2, true)  => Color32::from_rgb(85, 255, 85),     // Bright Green
            (3, false) => Color32::from_rgb(187, 187, 0),     // Yellow
            (3, true)  => Color32::from_rgb(255, 255, 85),    // Bright Yellow
            (4, false) => Color32::from_rgb(0, 0, 187),       // Blue
            (4, true)  => Color32::from_rgb(85, 85, 255),     // Bright Blue
            (5, false) => Color32::from_rgb(187, 0, 187),     // Magenta
            (5, true)  => Color32::from_rgb(255, 85, 255),    // Bright Magenta
            (6, false) => Color32::from_rgb(0, 187, 187),     // Cyan
            (6, true)  => Color32::from_rgb(85, 255, 255),    // Bright Cyan
            (7, false) => Color32::from_rgb(187, 187, 187),   // White
            (7, true)  => Color32::from_rgb(255, 255, 255),   // Bright White
            _ => Color32::LIGHT_GRAY,
        }
    }

    fn get_256_color(&self, index: u8) -> Color32 {
        if index < 8 {
            self.get_basic_color(index, false)
        } else if index < 16 {
            self.get_basic_color(index - 8, true)
        } else if index < 232 {
            // 6x6x6 color cube
            let i = index - 16;
            let r = (i / 36) * 51;
            let g = ((i / 6) % 6) * 51;
            let b = (i % 6) * 51;
            Color32::from_rgb(r, g, b)
        } else {
            // Grayscale ramp
            let gray = (index - 232) * 10 + 8;
            Color32::from_rgb(gray, gray, gray)
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
        fg_color: state.current_fg(),
        bg_color: state.current_bg(),
        blink: state.blink,
    };

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next(); // 消耗 '['
                    
                    // 1. 提取序列內容直至終止符
                    let mut sequence_content = String::new();
                    let mut cmd = '\0';
                    while let Some(&ch) = chars.peek() {
                        let b = ch as u8;
                        if (0x40..=0x7E).contains(&b) {
                            cmd = chars.next().unwrap();
                            break;
                        }
                        sequence_content.push(chars.next().unwrap());
                    }

                    // 2. 處理 SGR (顏色/樣式)
                    if cmd == 'm' {
                        if !current_span.text.is_empty() {
                            spans.push(current_span);
                        }

                        let mut params = Vec::new();
                        for part in sequence_content.split(';') {
                            let mut filtered = String::new();
                            for digit in part.chars().filter(|c| c.is_ascii_digit()) {
                                filtered.push(digit);
                            }
                            if let Ok(p) = filtered.parse::<u16>() {
                                params.push(p);
                            }
                        }

                        if params.is_empty() && sequence_content.is_empty() {
                            state.reset();
                        } else {
                            state.apply_code(&params);
                        }

                        current_span = AnsiSpan {
                            text: String::new(),
                            fg_color: state.current_fg(),
                            bg_color: state.current_bg(),
                            blink: state.blink,
                        };
                    } else if cmd != '\0' {
                        // 非 SGR 指令被忽略，cmd 與 sequence_content 都已從 chars 中消耗
                        // 且不會進入 else 分支加入 current_span.text
                    }
                }
                Some(&'(') | Some(&')') => {
                    chars.next(); // 消耗 '(' 或 ')'
                    chars.next(); // 消耗字集識別碼
                }
                _ => {
                    // 其他 ESC 序列暫不處理
                }
            }
        } else {
            // 基本字元處理
            if c >= ' ' || c == '\n' || c == '\r' || c == '\t' {
                current_span.text.push(c);
            }
        }
    }

    // 添加最後一個 span
    if !current_span.text.is_empty() {
        spans.push(current_span);
    }

    spans
}

/// 移除 ANSI 轉義碼，只保留純文字
/// 用於清理從畫面複製的文字，避免發送帶有控制碼的訊息
pub fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // 跳過 ANSI 序列
            match chars.peek() {
                Some(&'[') => {
                    chars.next(); // 消耗 '['
                    // 跳過直至終止符 (0x40-0x7E)
                    while let Some(&ch) = chars.peek() {
                        let b = ch as u8;
                        chars.next();
                        if (0x40..=0x7E).contains(&b) {
                            break;
                        }
                    }
                }
                Some(&'(') | Some(&')') => {
                    chars.next(); // 消耗 '(' 或 ')'
                    chars.next(); // 消耗字集識別碼
                }
                _ => {
                    // 其他 ESC 序列，跳過 ESC 本身
                }
            }
        } else if c >= ' ' || c == '\n' || c == '\r' || c == '\t' {
            // 只保留可見字元和基本空白字元
            result.push(c);
        }
        // 其他不可見控制字元被忽略
    }

    result
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

    #[test]
    fn test_background_color() {
        let spans = parse_ansi("\x1b[41mRed BG\x1b[0m");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Red BG");
        assert_eq!(spans[0].bg_color, Some(Color32::from_rgb(187, 0, 0)));
    }

    #[test]
    fn test_complex_csi_sequences() {
        // [3B 游標下移（忽略），接 [33;36;40m 顏色序列
        let input = "\x1b[3B\x1b[33;36;40mColor\x1b[0m";
        let spans = parse_ansi(input);
        for span in &spans {
            println!("Span: {:?}", span);
        }
        // 應產出 1 個 span: "Color"（顏色狀態已更新，但文字不含轉義雜訊）
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Color");
    }
}
