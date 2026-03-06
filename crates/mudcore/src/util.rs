/// 移除 ANSI 轉義碼（CSI 序列）
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
        assert_eq!(strip_ansi("No ANSI"), "No ANSI");
        assert_eq!(strip_ansi("\x1b[1;33m粗體黃\x1b[0m"), "粗體黃");
        assert_eq!(strip_ansi(""), "");
    }
}
