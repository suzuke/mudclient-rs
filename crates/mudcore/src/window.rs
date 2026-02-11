//! 多視窗管理模組
//!
//! 支援將訊息路由到不同的子視窗

use std::collections::HashMap;

/// 子視窗 ID
pub type WindowId = String;

/// 視窗訊息
#[derive(Debug, Clone)]
pub struct WindowMessage {
    /// 訊息內容
    pub content: String,
    /// 是否保留 ANSI 顏色
    pub preserve_ansi: bool,
    /// 原始位元組寬度映射 (選填)
    pub byte_widths: Vec<u8>,
}

impl WindowMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            preserve_ansi: true,
            byte_widths: Vec::new(),
        }
    }
    
    pub fn with_widths(mut self, widths: Vec<u8>) -> Self {
        self.byte_widths = widths;
        self
    }
}

/// 子視窗定義
#[derive(Debug, Clone)]
pub struct SubWindow {
    /// 視窗 ID
    pub id: WindowId,
    /// 視窗標題
    pub title: String,
    /// 訊息緩衝區容量
    pub capacity: usize,
    /// 訊息緩衝區
    messages: Vec<WindowMessage>,
    /// 是否可見
    pub visible: bool,
}

impl SubWindow {
    /// 創建新的子視窗
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            capacity: 1000,
            messages: Vec::new(),
            visible: true,
        }
    }

    /// 設置緩衝區容量
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// 添加訊息
    pub fn push(&mut self, message: WindowMessage) {
        if self.messages.len() >= self.capacity {
            self.messages.remove(0);
        }
        self.messages.push(message);
    }

    /// 獲取所有訊息
    pub fn messages(&self) -> &[WindowMessage] {
        &self.messages
    }

    /// 清空訊息
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// 獲取最後 N 條訊息
    pub fn last_n(&self, n: usize) -> &[WindowMessage] {
        let len = self.messages.len();
        if n >= len {
            &self.messages
        } else {
            &self.messages[len - n..]
        }
    }
}

/// 視窗管理器
#[derive(Debug, Default)]
pub struct WindowManager {
    /// 子視窗
    windows: HashMap<WindowId, SubWindow>,
    /// 視窗順序
    order: Vec<WindowId>,
    /// 主視窗 ID
    main_window_id: WindowId,
}

impl WindowManager {
    /// 創建新的視窗管理器
    pub fn new() -> Self {
        let main_id = "main".to_string();
        let mut manager = Self {
            windows: HashMap::new(),
            order: vec![main_id.clone()],
            main_window_id: main_id.clone(),
        };
        
        // 創建主視窗
        manager.windows.insert(
            main_id.clone(),
            SubWindow::new(main_id, "主視窗").with_capacity(10000),
        );
        
        manager
    }

    /// 添加子視窗
    pub fn add_window(&mut self, window: SubWindow) {
        let id = window.id.clone();
        if !self.windows.contains_key(&id) {
            self.order.push(id.clone());
        }
        self.windows.insert(id, window);
    }

    /// 移除子視窗
    pub fn remove_window(&mut self, id: &str) -> Option<SubWindow> {
        if id == self.main_window_id {
            return None; // 不能移除主視窗
        }
        self.order.retain(|x| x != id);
        self.windows.remove(id)
    }

    /// 獲取視窗
    pub fn get(&self, id: &str) -> Option<&SubWindow> {
        self.windows.get(id)
    }

    /// 獲取可變視窗
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SubWindow> {
        self.windows.get_mut(id)
    }

    /// 獲取主視窗
    pub fn main_window(&self) -> &SubWindow {
        self.windows.get(&self.main_window_id).unwrap()
    }

    /// 獲取可變主視窗
    pub fn main_window_mut(&mut self) -> &mut SubWindow {
        self.windows.get_mut(&self.main_window_id).unwrap()
    }

    /// 獲取所有視窗（按順序）
    pub fn windows(&self) -> Vec<&SubWindow> {
        self.order
            .iter()
            .filter_map(|id| self.windows.get(id))
            .collect()
    }

    /// 路由訊息到指定視窗 (帶寬度資訊)
    pub fn route_message_with_widths(&mut self, window_id: &str, message: WindowMessage) {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.push(message);
        } else {
            self.main_window_mut().push(message);
        }
    }

    /// 路由訊息到指定視窗
    pub fn route_message(&mut self, window_id: &str, message: WindowMessage) {
        self.route_message_with_widths(window_id, message);
    }

    /// 發送訊息到主視窗
    pub fn send_to_main(&mut self, content: impl Into<String>) {
        self.main_window_mut().push(WindowMessage {
            content: content.into(),
            preserve_ansi: true,
            byte_widths: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_manager_creation() {
        let manager = WindowManager::new();
        assert!(manager.get("main").is_some());
        assert_eq!(manager.windows().len(), 1);
    }

    #[test]
    fn test_add_sub_window() {
        let mut manager = WindowManager::new();
        manager.add_window(SubWindow::new("chat", "聊天"));
        
        assert!(manager.get("chat").is_some());
        assert_eq!(manager.windows().len(), 2);
    }

    #[test]
    fn test_route_message() {
        let mut manager = WindowManager::new();
        manager.add_window(SubWindow::new("chat", "聊天"));
        
        manager.route_message("chat", WindowMessage {
            content: "Hello".to_string(),
            preserve_ansi: true,
            byte_widths: Vec::new(),
        });
        
        assert_eq!(manager.get("chat").unwrap().messages().len(), 1);
    }

    #[test]
    fn test_cannot_remove_main_window() {
        let mut manager = WindowManager::new();
        assert!(manager.remove_window("main").is_none());
    }

    #[test]
    fn test_window_capacity() {
        let mut window = SubWindow::new("test", "Test").with_capacity(3);
        
        for i in 0..5 {
            window.push(WindowMessage {
                content: format!("Message {}", i),
                preserve_ansi: false,
                byte_widths: Vec::new(),
            });
        }
        
        assert_eq!(window.messages().len(), 3);
        assert_eq!(window.messages()[0].content, "Message 2");
    }
}
