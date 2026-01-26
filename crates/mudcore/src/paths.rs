use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 路徑定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    /// 直徑名稱 (使用者輸入此名稱時觸發)
    /// 例如: "market" -> 輸入 "market" 展開為 value
    pub name: String,
    
    /// 路徑內容 (peedwalk 格式或純指令序列)
    /// 建議使用 `/3w2n` 格式以便自動解析
    pub value: String,
    
    /// 分類 (用於 UI 分組)
    pub category: Option<String>,
}

impl Path {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            category: None,
        }
    }
}

/// 路徑管理器
#[derive(Debug, Default, Clone)]
pub struct PathManager {
    /// 名稱 -> 路徑 的映射
    paths: HashMap<String, Path>,
    
    /// 排序後的鍵值 (用於穩定輸出或 UI 顯示)
    pub sorted_keys: Vec<String>,
}

impl PathManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 新增或更新路徑
    pub fn add(&mut self, path: Path) {
        let name = path.name.clone();
        self.paths.insert(name, path);
        self.rebuild_index();
    }

    /// 移除路徑
    pub fn remove(&mut self, name: &str) -> Option<Path> {
        let res = self.paths.remove(name);
        if res.is_some() {
            self.rebuild_index();
        }
        res
    }

    /// 取得路徑
    pub fn get(&self, name: &str) -> Option<&Path> {
        self.paths.get(name)
    }

    /// 取得所有路徑 (已排序)
    pub fn list(&self) -> Vec<&Path> {
        self.sorted_keys
            .iter()
            .filter_map(|k| self.paths.get(k))
            .collect()
    }

    /// 清空所有路徑
    pub fn clear(&mut self) {
        self.paths.clear();
        self.sorted_keys.clear();
    }
    
    fn rebuild_index(&mut self) {
        let mut keys: Vec<_> = self.paths.keys().cloned().collect();
        keys.sort();
        self.sorted_keys = keys;
    }
}
