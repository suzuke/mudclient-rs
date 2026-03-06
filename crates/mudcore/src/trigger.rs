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
    /// 分類
    pub category: Option<String>,
    /// 匹配模式
    pub pattern: TriggerPattern,
    /// 執行動作列表
    pub actions: Vec<TriggerAction>,
    /// 是否啟用
    pub enabled: bool,
    /// 觸發器群組
    pub group: Option<String>,
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
            category: None,
            pattern,
            actions: Vec::new(),
            enabled: true,
            group: None,
            compiled_regex: compiled,
        }
    }

    /// 設定分類
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// 設定群組
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
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
        if !self.order.contains(&trigger.name) {
            self.order.push(trigger.name.clone());
        }
        let key = trigger.name.clone();
        self.triggers.insert(key, trigger);
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
        let stripped = Self::strip_ansi(message);
        self.process_pre_stripped(&stripped)
    }

    /// 處理已去除 ANSI 碼的訊息（避免重複 strip）
    pub fn process_pre_stripped(&self, stripped: &str) -> Vec<(&Trigger, TriggerMatch)> {
        let mut matches = Vec::new();

        for name in &self.order {
            if let Some(trigger) = self.triggers.get(name) {
                if let Some(m) = trigger.try_match(stripped) {
                    matches.push((trigger, m));
                }
            }
        }

        matches
    }

    /// 移除 ANSI 轉義碼（委託至 crate::util::strip_ansi）
    pub fn strip_ansi(input: &str) -> String {
        crate::util::strip_ansi(input)
    }

    /// 收集需要發送的命令
    pub fn collect_commands(&self, message: &str) -> Vec<String> {
        let stripped = Self::strip_ansi(message);
        self.collect_commands_pre_stripped(&stripped)
    }

    /// 收集需要發送的命令（已去除 ANSI 碼版本）
    pub fn collect_commands_pre_stripped(&self, stripped: &str) -> Vec<String> {
        let mut commands = Vec::new();

        for (trigger, m) in self.process_pre_stripped(stripped) {
            for action in &trigger.actions {
                if let TriggerAction::SendCommand(cmd) = action {
                    let mut expanded = cmd.clone();
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
        let stripped = Self::strip_ansi(message);
        self.should_gag_pre_stripped(&stripped)
    }

    /// Enable or disable all triggers in a group. Returns count affected.
    pub fn enable_group(&mut self, group: &str, enabled: bool) -> usize {
        let mut count = 0;
        for trigger in self.triggers.values_mut() {
            if trigger.group.as_deref() == Some(group) {
                trigger.enabled = enabled;
                count += 1;
            }
        }
        count
    }

    /// List all unique group names
    pub fn groups(&self) -> Vec<String> {
        let mut groups: Vec<String> = self.triggers.values()
            .filter_map(|t| t.group.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        groups.sort();
        groups
    }

    /// 檢查訊息是否應該被抑制（已去除 ANSI 碼版本）
    pub fn should_gag_pre_stripped(&self, stripped: &str) -> bool {
        for (trigger, _) in self.process_pre_stripped(stripped) {
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

    #[test]
    fn test_process_with_ansi_codes() {
        let mut manager = TriggerManager::new();
        manager.add(
            Trigger::new("hp", TriggerPattern::Contains("你受傷了".to_string()))
                .add_action(TriggerAction::SendCommand("heal".to_string())),
        );

        // ANSI colored text should still match after stripping
        let ansi_text = "\x1b[31m你受傷了\x1b[0m！";
        let matches = manager.process(ansi_text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.trigger_name, "hp");
    }

    #[test]
    fn test_process_pre_stripped() {
        let mut manager = TriggerManager::new();
        manager.add(
            Trigger::new("gold", TriggerPattern::Regex(r"獲得\s*(\d+)\s*金".to_string()))
                .add_action(TriggerAction::SendCommand("echo $1".to_string())),
        );
        manager.add(
            Trigger::new("gag", TriggerPattern::Contains("廣告".to_string()))
                .add_action(TriggerAction::Gag),
        );

        // Pre-stripped text should produce same results as process()
        let ansi_text = "\x1b[33m你獲得 50 金幣\x1b[0m";
        let stripped = TriggerManager::strip_ansi(ansi_text);
        let matches_normal = manager.process(ansi_text);
        let matches_pre = manager.process_pre_stripped(&stripped);
        assert_eq!(matches_normal.len(), matches_pre.len());
        assert_eq!(matches_normal[0].1.captures, matches_pre[0].1.captures);
    }

    #[test]
    fn test_strip_ansi() {
        assert_eq!(TriggerManager::strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
        assert_eq!(TriggerManager::strip_ansi("No ANSI"), "No ANSI");
        assert_eq!(TriggerManager::strip_ansi("\x1b[1;33m粗體黃\x1b[0m"), "粗體黃");
        assert_eq!(TriggerManager::strip_ansi(""), "");
    }

    #[test]
    fn test_should_gag_pre_stripped() {
        let mut manager = TriggerManager::new();
        manager.add(
            Trigger::new("gag_spam", TriggerPattern::Contains("廣告".to_string()))
                .add_action(TriggerAction::Gag),
        );

        let stripped = "這是一則廣告訊息";
        assert!(manager.should_gag_pre_stripped(stripped));
        assert!(!manager.should_gag_pre_stripped("正常訊息"));
    }

    #[test]
    fn test_trigger_groups() {
        let mut mgr = TriggerManager::new();
        let t1 = Trigger::new("t1", TriggerPattern::Contains("a".into())).with_group("combat");
        let t2 = Trigger::new("t2", TriggerPattern::Contains("b".into())).with_group("combat");
        let t3 = Trigger::new("t3", TriggerPattern::Contains("c".into()));
        mgr.add(t1);
        mgr.add(t2);
        mgr.add(t3);

        assert_eq!(mgr.enable_group("combat", false), 2);
        assert!(!mgr.get("t1").unwrap().enabled);
        assert!(!mgr.get("t2").unwrap().enabled);
        assert!(mgr.get("t3").unwrap().enabled);

        assert_eq!(mgr.enable_group("combat", true), 2);
        assert!(mgr.get("t1").unwrap().enabled);
    }

    #[test]
    fn test_groups_list() {
        let mut mgr = TriggerManager::new();
        mgr.add(Trigger::new("t1", TriggerPattern::Contains("a".into())).with_group("combat"));
        mgr.add(Trigger::new("t2", TriggerPattern::Contains("b".into())).with_group("explore"));
        mgr.add(Trigger::new("t3", TriggerPattern::Contains("c".into())));
        let groups = mgr.groups();
        assert_eq!(groups.len(), 2);
        assert!(groups.contains(&"combat".to_string()));
        assert!(groups.contains(&"explore".to_string()));
    }

    #[test]
    fn test_collect_commands_pre_stripped() {
        let mut manager = TriggerManager::new();
        manager.add(
            Trigger::new("gold", TriggerPattern::Regex(r"獲得\s*(\d+)\s*金".to_string()))
                .add_action(TriggerAction::SendCommand("echo $1 gold".to_string())),
        );

        let stripped = "你獲得 50 金幣";
        let commands = manager.collect_commands_pre_stripped(stripped);
        assert_eq!(commands, vec!["echo 50 gold"]);
    }
}
