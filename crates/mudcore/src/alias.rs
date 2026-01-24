//! Alias（別名）模組
//!
//! 將簡短輸入展開為完整命令

use regex::Regex;
use std::collections::HashMap;

/// 別名定義
#[derive(Debug, Clone)]
pub struct Alias {
    /// 別名名稱（用於識別）
    pub name: String,
    /// 分類
    pub category: Option<String>,
    /// 匹配模式（支援 $1, $2 等參數佔位符）
    pub pattern: String,
    /// 展開後的命令
    pub replacement: String,
    /// 是否啟用
    pub enabled: bool,
    /// 編譯後的正則表達式（內部使用）
    #[allow(dead_code)]
    compiled_regex: Option<Regex>,
}

impl Alias {
    /// 創建新的別名
    pub fn new(name: impl Into<String>, pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        let pattern = pattern.into();
        let regex = Self::compile_pattern(&pattern);
        
        Self {
            name: name.into(),
            category: None,
            pattern,
            replacement: replacement.into(),
            enabled: true,
            compiled_regex: regex,
        }
    }

    /// 設定分類
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// 將別名模式編譯為正則表達式
    fn compile_pattern(pattern: &str) -> Option<Regex> {
        // 轉義特殊字符，但保留 $1, $2 等參數佔位符
        let mut regex_pattern = String::from("^");
        let mut chars = pattern.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() {
                        // $1, $2 等 -> 捕獲組
                        regex_pattern.push_str(r"(.+?)");
                        chars.next(); // 消耗數字
                        continue;
                    } else if next == '*' {
                        // $* -> 匹配所有剩餘參數
                        regex_pattern.push_str(r"(.*)");
                        chars.next();
                        continue;
                    }
                }
            }
            
            // 轉義正則特殊字符
            match c {
                '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '\\' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(c);
                }
                '*' => {
                    regex_pattern.push_str(".*");
                }
                _ => {
                    regex_pattern.push(c);
                }
            }
        }
        
        regex_pattern.push('$');
        Regex::new(&regex_pattern).ok()
    }

    /// 嘗試匹配輸入並展開別名
    pub fn try_expand(&self, input: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let regex = self.compiled_regex.as_ref()?;
        let captures = regex.captures(input)?;

        let mut result = self.replacement.clone();
        
        // 替換 $1, $2 等佔位符
        for i in 1..captures.len() {
            if let Some(m) = captures.get(i) {
                let placeholder = format!("${}", i);
                result = result.replace(&placeholder, m.as_str());
            }
        }
        
        // 替換 $* （所有參數）
        if result.contains("$*") {
            let all_args: String = (1..captures.len())
                .filter_map(|i| captures.get(i).map(|m| m.as_str()))
                .collect::<Vec<_>>()
                .join(" ");
            result = result.replace("$*", &all_args);
        }

        Some(result)
    }
}

/// 別名管理器
#[derive(Debug, Default)]
pub struct AliasManager {
    pub aliases: HashMap<String, Alias>,
    /// 按優先級排序的別名列表（最長模式優先）
    pub sorted_aliases: Vec<String>,
}

impl AliasManager {
    /// 創建新的別名管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加別名
    pub fn add(&mut self, alias: Alias) {
        let name = alias.name.clone();
        self.aliases.insert(name.clone(), alias);
        self.rebuild_sorted_list();
    }

    /// 移除別名
    pub fn remove(&mut self, name: &str) -> Option<Alias> {
        let alias = self.aliases.remove(name);
        if alias.is_some() {
            self.rebuild_sorted_list();
        }
        alias
    }

    /// 獲取別名
    pub fn get(&self, name: &str) -> Option<&Alias> {
        self.aliases.get(name)
    }

    /// 獲取所有別名
    pub fn list(&self) -> Vec<&Alias> {
        self.aliases.values().collect()
    }

    /// 嘗試展開輸入
    pub fn expand(&self, input: &str) -> Option<String> {
        for name in &self.sorted_aliases {
            if let Some(alias) = self.aliases.get(name) {
                if let Some(expanded) = alias.try_expand(input) {
                    return Some(expanded);
                }
            }
        }
        None
    }

    /// 處理輸入：展開別名或返回原始輸入
    pub fn process(&self, input: &str) -> String {
        self.expand(input).unwrap_or_else(|| input.to_string())
    }

    /// 重建排序列表（按模式長度降序排列，確保更具體的匹配優先）
    fn rebuild_sorted_list(&mut self) {
        let mut list: Vec<_> = self.aliases.keys().cloned().collect();
        list.sort_by(|a, b| {
            let len_a = self.aliases.get(a).map(|x| x.pattern.len()).unwrap_or(0);
            let len_b = self.aliases.get(b).map(|x| x.pattern.len()).unwrap_or(0);
            len_b.cmp(&len_a) // 降序
        });
        self.sorted_aliases = list;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_alias() {
        let alias = Alias::new("kk", "kk", "kill kobold");
        assert_eq!(alias.try_expand("kk"), Some("kill kobold".to_string()));
        assert_eq!(alias.try_expand("kka"), None);
    }

    #[test]
    fn test_alias_with_parameter() {
        let alias = Alias::new("go", "go $1", "walk $1;look");
        assert_eq!(alias.try_expand("go north"), Some("walk north;look".to_string()));
        assert_eq!(alias.try_expand("go east"), Some("walk east;look".to_string()));
    }

    #[test]
    fn test_alias_with_multiple_parameters() {
        let alias = Alias::new("cast", "c $1 $2", "cast $1 at $2");
        assert_eq!(alias.try_expand("c fireball goblin"), Some("cast fireball at goblin".to_string()));
    }

    #[test]
    fn test_alias_with_all_params() {
        let alias = Alias::new("say", "s $*", "say $*");
        assert_eq!(alias.try_expand("s hello world"), Some("say hello world".to_string()));
    }

    #[test]
    fn test_alias_manager() {
        let mut manager = AliasManager::new();
        manager.add(Alias::new("kk", "kk", "kill kobold"));
        manager.add(Alias::new("go", "go $1", "walk $1;look"));

        assert_eq!(manager.process("kk"), "kill kobold");
        assert_eq!(manager.process("go north"), "walk north;look");
        assert_eq!(manager.process("hello"), "hello");
    }

    #[test]
    fn test_disabled_alias() {
        let mut alias = Alias::new("kk", "kk", "kill kobold");
        alias.enabled = false;
        assert_eq!(alias.try_expand("kk"), None);
    }

    #[test]
    fn test_alias_priority() {
        let mut manager = AliasManager::new();
        // 較長的模式應該優先匹配
        manager.add(Alias::new("go", "go $1", "walk $1"));
        manager.add(Alias::new("gonorth", "gonorth", "walk north;enter"));

        assert_eq!(manager.process("gonorth"), "walk north;enter");
        assert_eq!(manager.process("go south"), "walk south");
    }
}
