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

/// 迴圈偵測狀態
#[derive(Debug, Clone, PartialEq)]
pub enum LoopStatus {
    /// 未偵測到迴圈 (新地點)
    None,
    /// 確定迴圈 (Hash 部分相同且座標相同) - 100% 回到原點
    ExactLoop,
    /// 潛在迴圈 (Hash 相同但座標不同) - 可能是地形重複或傳送
    PotentialLoop,
}

/// 路徑記錄器
#[derive(Debug, Clone)]
pub struct PathRecorder {
    /// 是否正在記錄
    pub is_recording: bool,
    /// 已記錄的指令序列
    pub recorded_commands: Vec<String>,
    /// 目前相對座標 (x, y, z)
    pub current_pos: (i32, i32, i32),
    /// 已訪問過的地點 (Hash, Coordinates)
    pub visited_locations: Vec<(u64, (i32, i32, i32))>,
    /// 是否啟用迴圈偵測
    pub enable_loop_detection: bool,
}

impl Default for PathRecorder {
    fn default() -> Self {
        Self {
            is_recording: false,
            recorded_commands: Vec::new(),
            current_pos: (0, 0, 0),
            visited_locations: Vec::new(),
            enable_loop_detection: true,
        }
    }
}

impl PathRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 開始記錄
    pub fn start(&mut self) {
        self.is_recording = true;
        self.recorded_commands.clear();
        self.current_pos = (0, 0, 0);
        self.visited_locations.clear();
    }

    /// 停止記錄
    pub fn stop(&mut self) {
        self.is_recording = false;
    }

    /// 記錄房間特徵並檢查迴圈
    pub fn record_room(&mut self, content_hash: u64) -> LoopStatus {
        if !self.is_recording || !self.enable_loop_detection {
            return LoopStatus::None;
        }

        // 檢查是否曾經出現過這個 Hash
        let mut potential_match = false;
        
        for (hash, pos) in &self.visited_locations {
            if *hash == content_hash {
                if *pos == self.current_pos {
                    return LoopStatus::ExactLoop;
                }
                potential_match = true;
            }
        }

        // 記錄此地點
        self.visited_locations.push((content_hash, self.current_pos));

        if potential_match {
            LoopStatus::PotentialLoop
        } else {
            LoopStatus::None
        }
    }

    /// 嘗試記錄指令 (若是移動指令則記錄)
    pub fn record(&mut self, cmd: &str) -> bool {
        if !self.is_recording {
            return false;
        }

        let cmd = cmd.trim().to_lowercase();
        if self.is_movement_command(&cmd) {
            self.recorded_commands.push(cmd.clone());
            self.update_position(&cmd);
            return true;
        }
        false
    }
    
    /// 更新座標
    fn update_position(&mut self, cmd: &str) {
        let (x, y, z) = self.current_pos;
        match cmd {
            "n" | "north" => self.current_pos = (x, y + 1, z),
            "s" | "south" => self.current_pos = (x, y - 1, z),
            "e" | "east" => self.current_pos = (x + 1, y, z),
            "w" | "west" => self.current_pos = (x - 1, y, z),
            "u" | "up" => self.current_pos = (x, y, z + 1),
            "d" | "down" => self.current_pos = (x, y, z - 1),
            "ne" | "northeast" => self.current_pos = (x + 1, y + 1, z),
            "nw" | "northwest" => self.current_pos = (x - 1, y + 1, z),
            "se" | "southeast" => self.current_pos = (x + 1, y - 1, z),
            "sw" | "southwest" => self.current_pos = (x - 1, y - 1, z),
            _ => {} 
        }
    }
    
    /// 移除最後一步
    pub fn pop_last(&mut self) -> Option<String> {
        // undo 時暫不回溯座標，因為複雜度較高
        self.recorded_commands.pop()
    }

    /// 清除所有記錄
    pub fn clear(&mut self) {
        self.recorded_commands.clear();
        self.visited_locations.clear();
        self.current_pos = (0, 0, 0);
    }

    /// 取得目前路徑字串 (Speedwalk 格式)
    pub fn get_path_string(&self) -> String {
        // 簡單串接，未來可優化合併 (e.g., n, n, n -> 3n)
        self.recorded_commands.join(";")
    }

    /// 產生回溯路徑 (反向指令序列)
    pub fn get_reverse_path(&self) -> Vec<String> {
        let mut reverse_cmds = Vec::new();
        for cmd in self.recorded_commands.iter().rev() {
            if let Some(rev) = self.get_reverse_direction(cmd) {
                reverse_cmds.push(rev);
            }
        }
        reverse_cmds
    }

    fn is_movement_command(&self, cmd: &str) -> bool {
        matches!(
            cmd,
            "n" | "s" | "e" | "w" | "u" | "d" | 
            "ne" | "nw" | "se" | "sw" |
            "north" | "south" | "east" | "west" | "up" | "down" |
            "northeast" | "northwest" | "southeast" | "southwest"
        )
    }

    fn get_reverse_direction(&self, cmd: &str) -> Option<String> {
        match cmd {
            "n" => Some("s".to_string()),
            "s" => Some("n".to_string()),
            "e" => Some("w".to_string()),
            "w" => Some("e".to_string()),
            "u" => Some("d".to_string()),
            "d" => Some("u".to_string()),
            
            "ne" => Some("sw".to_string()),
            "sw" => Some("ne".to_string()),
            "nw" => Some("se".to_string()),
            "se" => Some("nw".to_string()),
            
            "north" => Some("south".to_string()),
            "south" => Some("north".to_string()),
            "east" => Some("west".to_string()),
            "west" => Some("east".to_string()),
            "up" => Some("down".to_string()),
            "down" => Some("up".to_string()),
            
            "northeast" => Some("southwest".to_string()),
            "southwest" => Some("northeast".to_string()),
            "northwest" => Some("southeast".to_string()),
            "southeast" => Some("northwest".to_string()),
            
            _ => None,
        }
    }

    /// 優化路徑：移除無效的來回移動 (e.g., n, s)
    pub fn simplify(&mut self) {
        let mut new_cmds: Vec<String> = Vec::new();
        
        for cmd in &self.recorded_commands {
            if let Some(last) = new_cmds.last() {
                // 檢查是否與上一步互為反向
                if let Some(rev) = self.get_reverse_direction(cmd) {
                     if *last == rev {
                         new_cmds.pop(); // 抵銷
                         continue;
                     }
                }
            }
            new_cmds.push(cmd.clone());
        }
        
        self.recorded_commands = new_cmds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_basic() {
        let mut recorder = PathRecorder::new();
        recorder.start();
        
        assert!(recorder.record("n"));
        assert!(recorder.record("e"));
        assert!(!recorder.record("look")); // 非移動指令
        
        assert_eq!(recorder.recorded_commands, vec!["n", "e"]);
        
        let rev = recorder.get_reverse_path();
        assert_eq!(rev, vec!["w", "s"]);
    }

    #[test]
    fn test_recorder_undo() {
        let mut recorder = PathRecorder::new();
        recorder.start();
        recorder.record("n");
        recorder.record("s");
        
        recorder.pop_last();
        assert_eq!(recorder.recorded_commands, vec!["n"]);
    }
    
    #[test]
    fn test_reverse_directions() {
        let recorder = PathRecorder::new();
        assert_eq!(recorder.get_reverse_direction("nw"), Some("se".to_string()));
        assert_eq!(recorder.get_reverse_direction("up"), Some("down".to_string()));
    }

    #[test]
    fn test_simplify_path() {
        let mut recorder = PathRecorder::new();
        recorder.start();
        // n, s -> 抵銷
        // e, e, w -> e
        recorder.record("n");
        recorder.record("s");
        recorder.record("e");
        recorder.record("e");
        recorder.record("w");
        
        recorder.simplify();
        assert_eq!(recorder.recorded_commands, vec!["e"]);
    }

    #[test]
    fn test_recorder_tracking() {
        let mut recorder = PathRecorder::new();
        recorder.start();
        recorder.record("n"); // (0, 1, 0)
        assert_eq!(recorder.current_pos, (0, 1, 0));
        recorder.record("e"); // (1, 1, 0)
        assert_eq!(recorder.current_pos, (1, 1, 0));
        recorder.record("u"); // (1, 1, 1)
        assert_eq!(recorder.current_pos, (1, 1, 1));
    }

    #[test]
    fn test_loop_detection() {
        let mut recorder = PathRecorder::new();
        recorder.start();
        
        // (0, 0, 0)
        let hash1 = 12345;
        assert_eq!(recorder.record_room(hash1), LoopStatus::None);
        
        recorder.record("n"); // (0, 1, 0)
        let hash2 = 67890;
        assert_eq!(recorder.record_room(hash2), LoopStatus::None);

        // Potential loop: Same hash, different pos
        recorder.record("n"); // (0, 2, 0)
        assert_eq!(recorder.record_room(hash1), LoopStatus::PotentialLoop);
        
        // Exact loop: Go back to start
        recorder.record("s");
        recorder.record("s"); // (0, 0, 0)
        assert_eq!(recorder.record_room(hash1), LoopStatus::ExactLoop);
    }
}
