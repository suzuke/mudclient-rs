//! Trigger（觸發器）模組
//!
//! 自動偵測訊息並執行動作

use regex::Regex;
use std::collections::HashMap;

/// 觸發器動作
#[derive(Debug, Clone)]
pub enum TriggerAction {
    /// 發送命令到 MUD
    SendCommand(String),
    /// 高亮顯示（前景色 RGB）
    Highlight { r: u8, g: u8, b: u8 },
    /// 抑制訊息（不顯示）
    Gag,
    /// 播放音效（路徑）
    PlaySound(String),
    /// 執行腳本
    ExecuteScript(String),
    /// 路由到子視窗
    RouteToWindow(String),
}

/// 觸發器匹配模式
#[derive(Debug, Clone)]
pub enum TriggerPattern {
    /// 純文字匹配（包含）
    Contains(String),
    /// 純文字匹配（開頭）
    StartsWith(String),
    /// 純文字匹配（結尾）
    EndsWith(String),
    /// 正則表達式
    Regex(String),
}

/// 觸發器定義
#[derive(Debug, Clone)]
pub struct Trigger {
    /// 觸發器名稱
    pub name: String,
    /// 匹配模式
    pub pattern: TriggerPattern,
    /// 執行動作列表
    pub actions: Vec<TriggerAction>,
    /// 是否啟用
    pub enabled: bool,
    /// 編譯後的正則（內部使用）
    compiled_regex: Option<Regex>,
}

impl Trigger {
    /// 創建新的觸發器
    pub fn new(name: impl Into<String>, pattern: TriggerPattern) -> Self {
        let compiled = match &pattern {
            TriggerPattern::Regex(re) => Regex::new(re).ok(),
            _ => None,
        };

        Self {
            name: name.into(),
            pattern,
            actions: Vec::new(),
            enabled: true,
            compiled_regex: compiled,
        }
    }

    /// 添加動作
    pub fn add_action(mut self, action: TriggerAction) -> Self {
        self.actions.push(action);
        self
    }

    /// 嘗試匹配訊息，返回捕獲的群組（如果有）
    pub fn try_match(&self, message: &str) -> Option<TriggerMatch> {
        if !self.enabled {
            return None;
        }

        match &self.pattern {
            TriggerPattern::Contains(s) => {
                if message.contains(s) {
                    Some(TriggerMatch {
                        trigger_name: self.name.clone(),
                        matched_text: s.clone(),
                        captures: vec![],
                    })
                } else {
                    None
                }
            }
            TriggerPattern::StartsWith(s) => {
                if message.starts_with(s) {
                    Some(TriggerMatch {
                        trigger_name: self.name.clone(),
                        matched_text: s.clone(),
                        captures: vec![],
                    })
                } else {
                    None
                }
            }
            TriggerPattern::EndsWith(s) => {
                if message.ends_with(s) {
                    Some(TriggerMatch {
                        trigger_name: self.name.clone(),
                        matched_text: s.clone(),
                        captures: vec![],
                    })
                } else {
                    None
                }
            }
            TriggerPattern::Regex(_) => {
                let regex = self.compiled_regex.as_ref()?;
                let captures = regex.captures(message)?;
                
                let groups: Vec<String> = captures
                    .iter()
                    .skip(1)
                    .filter_map(|m| m.map(|m| m.as_str().to_string()))
                    .collect();

                Some(TriggerMatch {
                    trigger_name: self.name.clone(),
                    matched_text: captures.get(0)?.as_str().to_string(),
                    captures: groups,
                })
            }
        }
    }
}

/// 觸發器匹配結果
#[derive(Debug, Clone)]
pub struct TriggerMatch {
    /// 觸發器名稱
    pub trigger_name: String,
    /// 匹配的文字
    pub matched_text: String,
    /// 捕獲的群組（僅 Regex 模式）
    pub captures: Vec<String>,
}

/// 觸發器管理器
#[derive(Debug, Default)]
pub struct TriggerManager {
    pub triggers: HashMap<String, Trigger>,
    /// 按優先級排序的觸發器列表
    pub order: Vec<String>,
}

impl TriggerManager {
    /// 創建新的觸發器管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加觸發器
    pub fn add(&mut self, trigger: Trigger) {
        let name = trigger.name.clone();
        self.triggers.insert(name.clone(), trigger);
        if !self.order.contains(&name) {
            self.order.push(name);
        }
    }

    /// 移除觸發器
    pub fn remove(&mut self, name: &str) -> Option<Trigger> {
        self.order.retain(|n| n != name);
        self.triggers.remove(name)
    }

    /// 獲取觸發器
    pub fn get(&self, name: &str) -> Option<&Trigger> {
        self.triggers.get(name)
    }

    /// 獲取可變觸發器
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Trigger> {
        self.triggers.get_mut(name)
    }

    /// 獲取所有觸發器
    pub fn list(&self) -> Vec<&Trigger> {
        self.order
            .iter()
            .filter_map(|name| self.triggers.get(name))
            .collect()
    }

    /// 處理訊息，返回所有匹配的觸發器及其動作
    pub fn process(&self, message: &str) -> Vec<(&Trigger, TriggerMatch)> {
        let mut matches = Vec::new();

        for name in &self.order {
            if let Some(trigger) = self.triggers.get(name) {
                if let Some(m) = trigger.try_match(message) {
                    matches.push((trigger, m));
                }
            }
        }

        matches
    }

    /// 收集需要發送的命令
    pub fn collect_commands(&self, message: &str) -> Vec<String> {
        let mut commands = Vec::new();

        for (trigger, m) in self.process(message) {
            for action in &trigger.actions {
                if let TriggerAction::SendCommand(cmd) = action {
                    let mut expanded = cmd.clone();
                    // 替換捕獲群組
                    for (i, cap) in m.captures.iter().enumerate() {
                        expanded = expanded.replace(&format!("${}", i + 1), cap);
                    }
                    commands.push(expanded);
                }
            }
        }

        commands
    }

    /// 檢查訊息是否應該被抑制（Gag）
    pub fn should_gag(&self, message: &str) -> bool {
        for (trigger, _) in self.process(message) {
            for action in &trigger.actions {
                if matches!(action, TriggerAction::Gag) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_trigger() {
        let trigger = Trigger::new("hp_warn", TriggerPattern::Contains("你受傷了".to_string()))
            .add_action(TriggerAction::Highlight { r: 255, g: 0, b: 0 });

        assert!(trigger.try_match("戰鬥中你受傷了！").is_some());
        assert!(trigger.try_match("你恢復了健康").is_none());
    }

    #[test]
    fn test_regex_trigger() {
        let trigger = Trigger::new(
            "gold",
            TriggerPattern::Regex(r"你獲得了\s*(\d+)\s*金幣".to_string()),
        )
        .add_action(TriggerAction::SendCommand("count gold".to_string()));

        let m = trigger.try_match("你獲得了 100 金幣！").unwrap();
        assert_eq!(m.captures, vec!["100".to_string()]);
    }

    #[test]
    fn test_trigger_manager() {
        let mut manager = TriggerManager::new();
        
        manager.add(
            Trigger::new("gold", TriggerPattern::Regex(r"獲得\s*(\d+)\s*金".to_string()))
                .add_action(TriggerAction::SendCommand("echo $1 gold".to_string())),
        );

        let commands = manager.collect_commands("你獲得 50 金幣");
        assert_eq!(commands, vec!["echo 50 gold"]);
    }

    #[test]
    fn test_gag_trigger() {
        let mut manager = TriggerManager::new();
        
        manager.add(
            Trigger::new("gag_spam", TriggerPattern::Contains("廣告".to_string()))
                .add_action(TriggerAction::Gag),
        );

        assert!(manager.should_gag("這是一則廣告訊息"));
        assert!(!manager.should_gag("正常訊息"));
    }

    #[test]
    fn test_disabled_trigger() {
        let mut trigger = Trigger::new("test", TriggerPattern::Contains("test".to_string()));
        trigger.enabled = false;

        assert!(trigger.try_match("this is a test").is_none());
    }

    #[test]
    fn test_multiple_triggers() {
        let mut manager = TriggerManager::new();
        
        manager.add(Trigger::new("a", TriggerPattern::Contains("你".to_string())));
        manager.add(Trigger::new("b", TriggerPattern::Contains("金".to_string())));

        let matches = manager.process("你獲得金幣");
        assert_eq!(matches.len(), 2);
    }
}
