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
    pub fg_color_left: Option<Color32>,
    pub blink: bool,
    pub bold: bool,
    /// 每個字元對應的原始編碼位元組數 (用於對齊校正)
    pub byte_widths: Vec<u8>,
}

impl Default for AnsiSpan {
    fn default() -> Self {
        Self {
            text: String::new(),
            fg_color: Color32::from_rgb(200, 200, 200),
            bg_color: None,
            fg_color_left: None,
            blink: false,
            bold: false,
            byte_widths: Vec::new(),
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

    // 已廢棄：亮度補償已移至渲染器統一處理
}

/// 解析 ANSI 轉義碼，返回帶顏色的文字片段
pub fn parse_ansi(input: &str) -> Vec<AnsiSpan> {
    parse_ansi_with_widths(input, None)
}

/// 帶有原始位元組寬度資訊的 ANSI 解析
pub fn parse_ansi_with_widths(input: &str, byte_widths: Option<&[u8]>) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let mut state = AnsiState::new();
    let mut current_span = AnsiSpan {
        text: String::new(),
        fg_color: state.current_fg(),
        bg_color: state.current_bg(),
        fg_color_left: None,
        blink: state.blink,
        bold: state.bold,
        byte_widths: Vec::new(),
    };

    let mut pending_fg_left: Option<Color32> = None;
    let mut chars = input.chars().peekable();
    let mut width_idx = 0;

    while let Some(c) = chars.next() {
        let current_w = byte_widths.and_then(|bw| bw.get(width_idx).copied()).unwrap_or(1);
        width_idx += 1;

        if c == '\x1b' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next(); // 消耗 '['
                    width_idx += 1;
                    
                    let mut sequence_content = String::new();
                    let mut cmd = '\0';
                    while let Some(&ch) = chars.peek() {
                        let b = ch as u8;
                        if (0x40..=0x7E).contains(&b) {
                            cmd = chars.next().unwrap();
                            width_idx += 1;
                            break;
                        }
                        sequence_content.push(chars.next().unwrap());
                        width_idx += 1;
                    }

                    if cmd == 'm' {
                        let was_empty = current_span.text.is_empty();
                        if !was_empty {
                            spans.push(current_span);
                            pending_fg_left = None;
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

                        let is_bare_reset = (params.is_empty() && sequence_content.is_empty())
                            || (params.len() == 1 && params[0] == 0);
                        
                        if is_bare_reset && was_empty {
                            pending_fg_left = Some(state.current_fg());
                            state.reset();
                        } else {
                            if params.is_empty() && sequence_content.is_empty() {
                                state.reset();
                            } else {
                                state.apply_code(&params);
                            }
                        }

                        current_span = AnsiSpan {
                            text: String::new(),
                            fg_color: state.current_fg(),
                            bg_color: state.current_bg(),
                            fg_color_left: pending_fg_left,
                            blink: state.blink,
                            bold: state.bold,
                            byte_widths: Vec::new(),
                        };
                    }
                }
                Some(&'(') | Some(&')') => {
                    chars.next(); // 消耗 '(' 或 ')'
                    width_idx += 1;
                    chars.next(); // 消耗字集識別碼
                    width_idx += 1;
                }
                _ => {}
            }
        } else {
            if c >= ' ' || c == '\n' || c == '\r' || c == '\t' {
                current_span.text.push(c);
                current_span.byte_widths.push(current_w);
            }
        }
    }

    // 添加最後一個 span
    if !current_span.text.is_empty() {
        spans.push(current_span);
    }

    spans
}

/// 移除 ANSI 轉義碼和其他不可見字符，只保留純文字
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
        } else if is_visible_char(c) {
            result.push(c);
        }
        // 其他不可見控制字元被忽略
    }

    result
}

/// 判斷字符是否為可見字符（應該保留）
fn is_visible_char(c: char) -> bool {
    // 允許的字符：
    // - 普通可見字符（>= 空格）
    // - 換行、回車、Tab
    // - 中文及其他語言字符
    
    // 排除的字符：
    // - ASCII 控制字符 (0x00-0x1F)，除了 \n \r \t
    // - DEL (0x7F)
    // - Unicode 控制字符 (U+0080-U+009F)
    // - 零寬度字符：
    //   - U+200B Zero Width Space
    //   - U+200C Zero Width Non-Joiner
    //   - U+200D Zero Width Joiner
    //   - U+FEFF Byte Order Mark / Zero Width No-Break Space
    //   - U+2060 Word Joiner
    // - 其他格式控制字符 (U+200E-U+200F, U+2028-U+202F, U+2066-U+206F)

    match c {
        // 允許基本空白字符
        '\n' | '\r' | '\t' => true,
        // 排除 ASCII 控制字符
        '\x00'..='\x1f' | '\x7f' => false,
        // 排除 C1 控制字符
        '\u{0080}'..='\u{009f}' => false,
        // 排除零寬度字符和格式控制字符
        '\u{200b}'..='\u{200f}' | // Zero width chars, LRM, RLM
        '\u{2028}'..='\u{202f}' | // Line/paragraph separators, embedding controls
        '\u{2060}'..='\u{206f}' | // Word joiner, invisible operators
        '\u{feff}' => false,      // BOM / ZWNBSP
        // 其他字符都允許
        _ => true,
    }
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

    #[test]
    fn test_dual_color_keeps_first_color() {
        // 雙色字模式：\x1b[31m\x1b[m蠻 → 紅色碼後跟裸重置，應保留紅色於左半部，右半部為預設
        let input = "\x1b[31m\x1b[m蠻\x1b[31m\x1b[m荒";
        let spans = parse_ansi(input);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "蠻");
        assert_eq!(spans[0].fg_color_left, Some(Color32::from_rgb(187, 0, 0))); // Red
        assert_eq!(spans[0].fg_color, Color32::from_rgb(200, 200, 200)); // Default reset
    }

    #[test]
    fn test_normal_reset_still_works() {
        // 正常的 reset（有文字後的重置）不應被跳過
        let input = "\x1b[31mRed\x1b[0m Normal";
        let spans = parse_ansi(input);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Red");
        assert_eq!(spans[0].fg_color, Color32::from_rgb(187, 0, 0));
        assert_eq!(spans[1].text, " Normal");
        assert_eq!(spans[1].fg_color, Color32::from_rgb(200, 200, 200)); // Default
    }

    #[test]
    fn test_dual_color_multi_sgr() {
        // 更複雜的雙色字：\x1b[1;31m\x1b[m\x1b[1m桃 → bold red + reset + bold
        // 第一層捕捉到 bold red，第二層 reset，第三層設 bold。
        // 結果應該是：左半 bold red，右半 bold white。
        let input = "\x1b[1;31m\x1b[m\x1b[1m桃";
        let spans = parse_ansi(input);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "桃");
        assert_eq!(spans[0].fg_color_left, Some(Color32::from_rgb(255, 85, 85))); // Bold Red
        assert_eq!(spans[0].fg_color, Color32::from_rgb(255, 255, 255)); // Bold White
    }
}
