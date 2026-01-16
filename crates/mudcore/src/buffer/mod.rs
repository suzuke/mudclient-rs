//! 訊息緩衝區模組
//!
//! 提供固定大小的環形緩衝區來儲存 MUD 訊息歷史

use std::collections::VecDeque;

/// 訊息緩衝區 - 使用環形緩衝區儲存歷史訊息
///
/// 當緩衝區滿了時，最舊的訊息會被移除
#[derive(Debug, Clone)]
pub struct MessageBuffer {
    messages: VecDeque<String>,
    capacity: usize,
}

impl MessageBuffer {
    /// 創建新的訊息緩衝區
    ///
    /// # Arguments
    /// * `capacity` - 緩衝區最大容量
    ///
    /// # Example
    /// ```
    /// use mudcore::buffer::MessageBuffer;
    ///
    /// let buffer = MessageBuffer::new(100);
    /// assert_eq!(buffer.len(), 0);
    /// ```
    pub fn new(capacity: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// 添加訊息到緩衝區
    ///
    /// 如果緩衝區已滿，最舊的訊息會被移除
    pub fn push(&mut self, message: String) {
        if self.messages.len() >= self.capacity {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }

    /// 獲取所有訊息的迭代器
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.messages.iter()
    }

    /// 獲取緩衝區中的訊息數量
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 檢查緩衝區是否為空
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// 清空緩衝區
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// 獲取最後 n 條訊息
    pub fn last_n(&self, n: usize) -> Vec<&String> {
        self.messages.iter().rev().take(n).collect::<Vec<_>>().into_iter().rev().collect()
    }
}

impl Default for MessageBuffer {
    fn default() -> Self {
        Self::new(1000) // 預設容量 1000 條訊息
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = MessageBuffer::new(10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_push_message() {
        let mut buffer = MessageBuffer::new(10);
        buffer.push("Hello".to_string());
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_capacity_overflow() {
        let mut buffer = MessageBuffer::new(3);
        buffer.push("1".to_string());
        buffer.push("2".to_string());
        buffer.push("3".to_string());
        buffer.push("4".to_string()); // 這會移除 "1"

        assert_eq!(buffer.len(), 3);
        let messages: Vec<_> = buffer.iter().collect();
        assert_eq!(messages, vec!["2", "3", "4"]);
    }

    #[test]
    fn test_iter_order() {
        let mut buffer = MessageBuffer::new(10);
        buffer.push("first".to_string());
        buffer.push("second".to_string());
        buffer.push("third".to_string());

        let messages: Vec<_> = buffer.iter().collect();
        assert_eq!(messages, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_last_n() {
        let mut buffer = MessageBuffer::new(10);
        for i in 1..=5 {
            buffer.push(i.to_string());
        }

        let last_3 = buffer.last_n(3);
        assert_eq!(last_3, vec!["3", "4", "5"]);
    }

    #[test]
    fn test_clear() {
        let mut buffer = MessageBuffer::new(10);
        buffer.push("test".to_string());
        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_default() {
        let buffer = MessageBuffer::default();
        assert_eq!(buffer.capacity, 1000);
    }
}
