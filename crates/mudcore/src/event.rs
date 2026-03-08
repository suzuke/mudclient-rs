//! EventBus（事件匯流排）模組
//!
//! 提供事件驅動的 pub/sub 機制，支援 Lua handler 註冊與觸發

use std::collections::HashMap;
use std::time::Instant;
use mlua::RegistryKey;
use crate::script::LuaCallback;

/// Handler 唯一識別碼
pub type HandlerId = u64;

/// 事件資料
#[derive(Debug, Clone)]
pub struct Event {
    /// 事件名稱
    pub name: String,
    /// JSON 格式的事件資料
    pub data: Option<String>,
}

/// 事件處理器
pub struct EventHandler {
    /// 唯一識別碼
    pub id: HandlerId,
    /// Lua 回呼：字串程式碼或 function reference
    pub callback: LuaCallback,
    /// 優先序（數字越小越先執行）
    pub priority: i32,
    /// 是否只觸發一次
    pub once: bool,
    /// 是否啟用
    pub enabled: bool,
}

impl std::fmt::Debug for EventHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandler")
            .field("id", &self.id)
            .field("callback", &self.callback)
            .field("priority", &self.priority)
            .field("once", &self.once)
            .field("enabled", &self.enabled)
            .finish()
    }
}

/// emit() 回傳的單一 callback 項目
pub enum EmittedCallback {
    /// 字串程式碼（從 handler clone）
    Code(String),
    /// 擁有的 function key（once handler 被移除後轉移所有權）
    OwnedFunction(RegistryKey),
    /// 持久性 function handler，caller 需透過 get_handler_callback() 借用
    PersistentFunction,
}

impl std::fmt::Debug for EmittedCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmittedCallback::Code(s) => write!(f, "Code({:?})", s),
            EmittedCallback::OwnedFunction(_) => write!(f, "OwnedFunction(RegistryKey)"),
            EmittedCallback::PersistentFunction => write!(f, "PersistentFunction"),
        }
    }
}

/// 事件匯流排
pub struct EventBus {
    /// 事件名稱 → 已排序的 handler 列表
    handlers: HashMap<String, Vec<EventHandler>>,
    /// 下一個 handler ID
    next_id: HandlerId,
    /// 最近事件紀錄（用於 debug panel），最多保留 100 筆
    event_log: Vec<(Instant, String, Option<String>)>,
}

/// 事件紀錄最大容量
const EVENT_LOG_CAPACITY: usize = 100;

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            next_id: 1,
            event_log: Vec::new(),
        }
    }

    /// 註冊事件 handler，回傳 HandlerId
    pub fn on(
        &mut self,
        event_name: impl Into<String>,
        callback: LuaCallback,
        priority: i32,
        once: bool,
    ) -> HandlerId {
        let id = self.next_id;
        self.next_id += 1;

        let handler = EventHandler {
            id,
            callback,
            priority,
            once,
            enabled: true,
        };

        let handlers = self.handlers.entry(event_name.into()).or_default();
        handlers.push(handler);
        handlers.sort_by_key(|h| h.priority);

        id
    }

    /// 移除指定 handler，回傳是否成功
    pub fn off(&mut self, handler_id: HandlerId) -> bool {
        for handlers in self.handlers.values_mut() {
            if let Some(pos) = handlers.iter().position(|h| h.id == handler_id) {
                handlers.remove(pos);
                return true;
            }
        }
        false
    }

    /// 觸發事件，回傳要執行的 (HandlerId, EmittedCallback) 列表
    ///
    /// - Code handlers: 回傳 clone 的字串
    /// - once Function handlers: 移除 handler 並轉移 RegistryKey 所有權
    /// - persistent Function handlers: 回傳 PersistentFunction，caller 需透過
    ///   get_handler_callback() 借用 RegistryKey 來執行
    pub fn emit(
        &mut self,
        event_name: impl Into<String>,
        data: Option<String>,
    ) -> Vec<(HandlerId, EmittedCallback)> {
        let event_name = event_name.into();

        // 記錄事件
        if self.event_log.len() >= EVENT_LOG_CAPACITY {
            self.event_log.remove(0);
        }
        self.event_log
            .push((Instant::now(), event_name.clone(), data.clone()));

        let handlers = match self.handlers.get_mut(&event_name) {
            Some(h) => h,
            None => return Vec::new(),
        };

        // 收集要執行的 handler ID 與類型（已按 priority 排序）
        let to_fire: Vec<(HandlerId, bool, bool)> = handlers
            .iter()
            .filter(|h| h.enabled)
            .map(|h| {
                let is_function = matches!(&h.callback, LuaCallback::Function(_));
                (h.id, h.once, is_function)
            })
            .collect();

        let mut result = Vec::with_capacity(to_fire.len());

        for (id, once, is_function) in to_fire {
            if once {
                // once handler: 從 vec 中移除並取得所有權
                if let Some(pos) = handlers.iter().position(|h| h.id == id) {
                    let handler = handlers.remove(pos);
                    match handler.callback {
                        LuaCallback::Code(code) => result.push((id, EmittedCallback::Code(code))),
                        LuaCallback::Function(key) => result.push((id, EmittedCallback::OwnedFunction(key))),
                    }
                }
            } else if is_function {
                // persistent function: caller 需透過 get_handler_callback 借用
                result.push((id, EmittedCallback::PersistentFunction));
            } else {
                // persistent code: clone 字串
                if let Some(handler) = handlers.iter().find(|h| h.id == id) {
                    if let LuaCallback::Code(ref code) = handler.callback {
                        result.push((id, EmittedCallback::Code(code.clone())));
                    }
                }
            }
        }

        result
    }

    /// 取得 handler 的 callback reference（用於 persistent function handlers）
    pub fn get_handler_callback(&self, handler_id: HandlerId) -> Option<&LuaCallback> {
        for handlers in self.handlers.values() {
            if let Some(h) = handlers.iter().find(|h| h.id == handler_id) {
                return Some(&h.callback);
            }
        }
        None
    }

    /// 取得所有已註冊的事件名稱
    pub fn registered_events(&self) -> Vec<String> {
        self.handlers
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// 取得指定事件的 handler 數量
    pub fn handler_count(&self, event_name: &str) -> usize {
        self.handlers.get(event_name).map_or(0, |v| v.len())
    }

    /// 清除所有 handler 與事件紀錄
    pub fn clear(&mut self) {
        self.handlers.clear();
        self.event_log.clear();
        self.next_id = 1;
    }

    /// 取得事件紀錄（用於 debug panel）
    pub fn event_log(&self) -> &[(Instant, String, Option<String>)] {
        &self.event_log
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_off() {
        let mut bus = EventBus::new();
        let id = bus.on("test.event", LuaCallback::Code("print('hello')".into()), 0, false);
        assert_eq!(bus.handler_count("test.event"), 1);

        assert!(bus.off(id));
        assert_eq!(bus.handler_count("test.event"), 0);

        // 移除不存在的 handler
        assert!(!bus.off(9999));
    }

    #[test]
    fn test_emit_returns_handlers() {
        let mut bus = EventBus::new();
        let id1 = bus.on("combat.hit", LuaCallback::Code("handle_low()".into()), 10, false);
        let id2 = bus.on("combat.hit", LuaCallback::Code("handle_high()".into()), 1, false);

        let result = bus.emit("combat.hit", Some(r#"{"damage":100}"#.to_string()));
        assert_eq!(result.len(), 2);
        // priority 1 先於 priority 10
        assert_eq!(result[0].0, id2);
        assert_eq!(result[1].0, id1);
    }

    #[test]
    fn test_once_handler_removed_after_emit() {
        let mut bus = EventBus::new();
        bus.on("login.complete", LuaCallback::Code("do_init()".into()), 0, true);

        let first = bus.emit("login.complete", None);
        assert_eq!(first.len(), 1);

        let second = bus.emit("login.complete", None);
        assert!(second.is_empty());
        assert_eq!(bus.handler_count("login.complete"), 0);
    }

    #[test]
    fn test_event_log() {
        let mut bus = EventBus::new();
        bus.emit("player.move", Some(r#"{"room":"town"}"#.to_string()));

        let log = bus.event_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].1, "player.move");
        assert_eq!(log[0].2, Some(r#"{"room":"town"}"#.to_string()));
    }

    #[test]
    fn test_priority_ordering() {
        let mut bus = EventBus::new();
        let id_low = bus.on("tick", LuaCallback::Code("low()".into()), 50, false);
        let id_high = bus.on("tick", LuaCallback::Code("high()".into()), -10, false);
        let id_mid = bus.on("tick", LuaCallback::Code("mid()".into()), 5, false);

        let result = bus.emit("tick", None);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, id_high); // -10
        assert_eq!(result[1].0, id_mid); // 5
        assert_eq!(result[2].0, id_low); // 50
    }
}
