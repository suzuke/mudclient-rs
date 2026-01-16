//! Big5/UTF-8 編解碼模組
//!
//! 處理台灣 MUD 伺服器常用的 Big5 編碼轉換

use encoding_rs::BIG5;

/// 將 Big5 編碼的位元組轉換為 UTF-8 字串
///
/// # Arguments
/// * `bytes` - Big5 編碼的位元組切片
///
/// # Returns
/// 轉換後的 UTF-8 字串（無效字元會被替換為 U+FFFD）
///
/// # Example
/// ```
/// use mudcore::encoding::decode_big5;
///
/// let big5_bytes = [0xb4, 0xfa, 0xa6, 0xb4]; // "測試" in Big5
/// let text = decode_big5(&big5_bytes);
/// assert!(!text.is_empty());
/// ```
pub fn decode_big5(bytes: &[u8]) -> String {
    let (decoded, _, _) = BIG5.decode(bytes);
    decoded.into_owned()
}

/// 將 UTF-8 字串轉換為 Big5 編碼的位元組
///
/// # Arguments
/// * `text` - UTF-8 字串
///
/// # Returns
/// Big5 編碼的位元組向量（無法編碼的字元會被替換）
///
/// # Example
/// ```
/// use mudcore::encoding::encode_big5;
///
/// let text = "你好";
/// let bytes = encode_big5(text);
/// assert!(!bytes.is_empty());
/// ```
pub fn encode_big5(text: &str) -> Vec<u8> {
    let (encoded, _, _) = BIG5.encode(text);
    encoded.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_big5_chinese() {
        // "你好" in Big5: 0xa7 0x41 0xa6 0x6e
        let big5_bytes = [0xa7, 0x41, 0xa6, 0x6e];
        let result = decode_big5(&big5_bytes);
        assert_eq!(result, "你好");
    }

    #[test]
    fn test_decode_big5_ascii() {
        let ascii_bytes = b"Hello World";
        let result = decode_big5(ascii_bytes);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_decode_big5_mixed() {
        // "Hi你好" - ASCII + Big5
        let mixed: Vec<u8> = [b"Hi".as_slice(), &[0xa7, 0x41, 0xa6, 0x6e]].concat();
        let result = decode_big5(&mixed);
        assert_eq!(result, "Hi你好");
    }

    #[test]
    fn test_encode_big5_chinese() {
        let text = "你好";
        let result = encode_big5(text);
        assert_eq!(result, vec![0xa7, 0x41, 0xa6, 0x6e]);
    }

    #[test]
    fn test_encode_big5_ascii() {
        let text = "Hello";
        let result = encode_big5(text);
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_roundtrip() {
        let original = "測試MUD客戶端";
        let encoded = encode_big5(original);
        let decoded = decode_big5(&encoded);
        assert_eq!(decoded, original);
    }
}
