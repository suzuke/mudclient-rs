use regex::Regex;
use std::sync::OnceLock;

/// 解析 Speedwalk路徑字串
///
/// 格式範例: `/3w3se` -> `recall`, `w`, `w`, `w`, `se`, `se`, `se`
///
/// 規則:
/// 1. 必須以 `/` 開頭 (代表 recall)
/// 2. 支援格式:
///    - 數字 (可選) + 方向 (n, s, e, w, ne, nw, se, sw, u, d)
///    - 方向不區分大小寫
///
/// 如果解析失敗或格式不符，回傳 None
pub fn parse_speedwalk(input: &str) -> Option<Vec<String>> {
    // 必須以 '/' 開頭
    if !input.starts_with('/') {
        return None;
    }

    // 移除開頭的 '/'，並加入 recall 指令
    let mut commands = vec!["recall".to_string()];
    let remainder = &input[1..];

    if remainder.is_empty() {
        return Some(commands);
    }

    // 正則表達式：匹配 (數字)? (方向)
    // 方向包含: n, s, e, w, ne, nw, se, sw, u, d
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)^(\d*)(ne|nw|se|sw|n|s|e|w|u|d)").unwrap()
    });

    // 使用 remainder slice 逐步解析
    let mut parsing_slice = remainder;

    while !parsing_slice.is_empty() {
        if let Some(captures) = re.captures(parsing_slice) {
            let full_match = captures.get(0).unwrap();
            let count_str = captures.get(1).map_or("", |m| m.as_str());
            let direction = captures.get(2).unwrap().as_str();

            let count: usize = if count_str.is_empty() {
                1
            } else {
                // 如果數字太大或解析失敗，視為 1 (或是這裡應該 fail? 照直覺先 parse)
                count_str.parse().unwrap_or(1) // Regex 確保是數字，應該不會 panic
            };

            for _ in 0..count {
                commands.push(direction.to_lowercase());
            }

            // 前進
            parsing_slice = &parsing_slice[full_match.end()..];
        } else {
            // 如果遇到無法解析的字元，視為無效路徑？
            // 還是忽略？ 為求嚴謹，若中間有垃圾字元，視為解析失敗可能較好，避免誤操作
            // 但如果使用者輸入 `/3w 2n` (有空格)，是否該支援？
            // 目前 regex 沒處理空格。
            // 嘗試跳過空格
            if let Some(first_char) = parsing_slice.chars().next() {
                 if first_char.is_whitespace() {
                     parsing_slice = &parsing_slice[1..];
                     continue;
                 }
            }
            
            // 遇到非空格且無法匹配方向的字元 -> 解析失敗
            return None;
        }
    }

    Some(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recall_only() {
        assert_eq!(parse_speedwalk("/"), Some(vec!["recall".to_string()]));
    }

    #[test]
    fn test_simple_path() {
        assert_eq!(
            parse_speedwalk("/n"),
            Some(vec!["recall".to_string(), "n".to_string()])
        );
    }

    #[test]
    fn test_numbered_path() {
        assert_eq!(
            parse_speedwalk("/3w"),
            Some(vec!["recall".to_string(), "w".to_string(), "w".to_string(), "w".to_string()])
        );
    }

    #[test]
    fn test_complex_path() {
        assert_eq!(
            parse_speedwalk("/2n3e1u"),
            Some(vec![
                "recall".to_string(),
                "n".to_string(), "n".to_string(),
                "e".to_string(), "e".to_string(), "e".to_string(),
                "u".to_string()
            ])
        );
    }

    #[test]
    fn test_directions_mixed() {
        assert_eq!(
            parse_speedwalk("/nwse"),
            Some(vec![
                "recall".to_string(),
                "nw".to_string(),
                "se".to_string()
            ])
        );
    }
    
    #[test]
    fn test_mixed_case() {
        assert_eq!(
            parse_speedwalk("/2N1Sw"),
            Some(vec![
                "recall".to_string(),
                "n".to_string(), "n".to_string(),
                "sw".to_string()
            ])
        );
    }

    #[test]
    fn test_invalid_prefix() {
        assert_eq!(parse_speedwalk("3w"), None);
    }

    #[test]
    fn test_invalid_path_char() {
        assert_eq!(parse_speedwalk("/3wx"), None); // x is invalid
    }
    
    #[test]
    fn test_ignore_whitespace() {
         assert_eq!(
            parse_speedwalk("/ 2n 3w "),
            Some(vec![
                "recall".to_string(),
                "n".to_string(), "n".to_string(),
                "w".to_string(), "w".to_string(), "w".to_string()
            ])
        );
    }
}
