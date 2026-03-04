//! MapDatabase: Rust 內建地圖記錄核心
//!
//! 提供房間記錄、邊緣追蹤、BFS 尋路、JSON 持久化。
//! JSON 格式與 Lua MudMapper 完全相容。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::Instant;

/// 房間記錄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub visited: u32,
    #[serde(default)]
    pub first_visit: u64,
    #[serde(default, deserialize_with = "super::room_deserialize_exits")]
    pub exits: Vec<String>,
}

/// 地圖資料庫
#[derive(Debug, Serialize, Deserialize)]
pub struct MapDatabase {
    pub rooms: HashMap<String, RoomRecord>,
    pub moves: HashMap<String, HashMap<String, String>>,

    #[serde(skip)]
    pub enabled: bool,
    #[serde(skip)]
    pub data_version: u64,
    #[serde(skip)]
    last_direction: Option<String>,
    #[serde(skip)]
    last_direction_time: Option<Instant>,
    #[serde(skip)]
    pub last_room_id: Option<String>,
}

impl Default for MapDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl MapDatabase {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            moves: HashMap::new(),
            enabled: true,
            data_version: 0,
            last_direction: None,
            last_direction_time: None,
            last_room_id: None,
        }
    }

    /// 記錄使用者輸入的移動方向
    pub fn record_last_direction(&mut self, cmd: &str) {
        if !self.enabled {
            return;
        }
        if let Some(normalized) = normalize_direction(cmd) {
            self.last_direction = Some(normalized);
            self.last_direction_time = Some(Instant::now());
        }
    }

    /// 偵測到房間時呼叫，記錄房間 + 邊緣
    pub fn on_room_detected(&mut self, id: &str, name: &str, exits: &[String]) {
        if !self.enabled {
            return;
        }

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let is_new = !self.rooms.contains_key(id);

        // 更新或新增房間
        let room = self.rooms.entry(id.to_string()).or_insert_with(|| RoomRecord {
            id: id.to_string(),
            name: name.to_string(),
            visited: 0,
            first_visit: now_epoch,
            exits: exits.to_vec(),
        });
        room.visited += 1;
        room.name = name.to_string();
        if !exits.is_empty() {
            room.exits = exits.to_vec();
        }

        // 建立邊緣（方向連接）
        if let (Some(last_id), Some(dir), Some(dir_time)) = (
            &self.last_room_id,
            &self.last_direction,
            self.last_direction_time,
        ) {
            let elapsed = dir_time.elapsed().as_secs();
            // 5 秒超時 + 自環防護
            if elapsed <= 5 && last_id != id {
                let edges = self.moves.entry(last_id.clone()).or_default();
                edges.insert(dir.clone(), id.to_string());
            }
        }

        self.last_room_id = Some(id.to_string());
        self.last_direction = None; // 消耗移動意圖
        self.data_version += 1;

        if is_new {
            let short_id: String = id.chars().take(8).collect();
            tracing::info!("[Map] 新房間: {} (ID: {}...)", name, short_id);
        }
    }

    /// BFS 尋路，回傳方向序列
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return Some(Vec::new());
        }

        let mut visited = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();

        visited.insert(from.to_string());
        queue.push_back((from.to_string(), Vec::new()));

        while let Some((current_id, path)) = queue.pop_front() {
            if let Some(exits) = self.moves.get(&current_id) {
                for (dir, next_id) in exits {
                    if !visited.contains(next_id) {
                        visited.insert(next_id.clone());
                        let mut new_path = path.clone();
                        new_path.push(dir.clone());

                        if next_id == to {
                            return Some(new_path);
                        }
                        queue.push_back((next_id.clone(), new_path));
                    }
                }
            }
        }

        None
    }

    /// 搜尋房間：精確 ID → 前綴 ID → 名稱包含
    pub fn resolve_target(&self, input: &str) -> Vec<(String, String)> {
        // 1. 精確 ID
        if let Some(room) = self.rooms.get(input) {
            return vec![(room.id.clone(), room.name.clone())];
        }

        // 2. 前綴 ID / 名稱搜尋
        let mut matches = Vec::new();
        for (id, room) in &self.rooms {
            if id.starts_with(input) || room.name.contains(input) {
                matches.push((id.clone(), room.name.clone()));
            }
        }
        matches
    }

    /// 儲存至 JSON 檔案（原子寫入：先寫暫存檔再 rename，防止中斷導致損毀）
    pub fn save(&self, path: &Path) -> Result<(), String> {
        // 確保目錄存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("建立目錄失敗: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("JSON 序列化失敗: {}", e))?;

        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)
            .map_err(|e| format!("寫入暫存檔失敗: {}", e))?;
        std::fs::rename(&tmp_path, path)
            .map_err(|e| format!("替換檔案失敗: {}", e))?;
        Ok(())
    }

    /// 從 JSON 檔案載入
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("讀取檔案失敗: {}", e))?;
        let mut db: MapDatabase = serde_json::from_str(&content)
            .map_err(|e| format!("JSON 解析失敗: {}", e))?;
        db.enabled = true;
        db.data_version = 1; // 標記已載入
        Ok(db)
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn edge_count(&self) -> usize {
        self.moves.values().map(|m| m.len()).sum()
    }
}

/// 判斷是否為方向指令
pub fn is_direction_command(cmd: &str) -> bool {
    normalize_direction(cmd).is_some()
}

/// 正規化方向指令，回傳短縮寫 (n/s/e/w/u/d/ne/nw/se/sw)
pub fn normalize_direction(cmd: &str) -> Option<String> {
    let s = cmd.trim().to_lowercase();
    match s.as_str() {
        "n" | "north" => Some("n".into()),
        "s" | "south" => Some("s".into()),
        "e" | "east" => Some("e".into()),
        "w" | "west" => Some("w".into()),
        "u" | "up" => Some("u".into()),
        "d" | "down" => Some("d".into()),
        "ne" | "northeast" => Some("ne".into()),
        "nw" | "northwest" => Some("nw".into()),
        "se" | "southeast" => Some("se".into()),
        "sw" | "southwest" => Some("sw".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_direction() {
        assert_eq!(normalize_direction("north"), Some("n".into()));
        assert_eq!(normalize_direction("Southeast"), Some("se".into()));
        assert_eq!(normalize_direction("look"), None);
        assert_eq!(normalize_direction("u"), Some("u".into()));
    }

    #[test]
    fn test_on_room_detected_records_room() {
        let mut db = MapDatabase::new();
        db.on_room_detected("room1", "大廳", &["north".into(), "south".into()]);
        assert_eq!(db.rooms.len(), 1);
        assert_eq!(db.rooms["room1"].name, "大廳");
        assert_eq!(db.rooms["room1"].visited, 1);
    }

    #[test]
    fn test_edge_creation() {
        let mut db = MapDatabase::new();
        db.on_room_detected("room1", "大廳", &["north".into()]);
        db.record_last_direction("north");
        db.on_room_detected("room2", "走廊", &["south".into()]);

        assert_eq!(db.moves.get("room1").unwrap().get("n").unwrap(), "room2");
    }

    #[test]
    fn test_no_self_loop() {
        let mut db = MapDatabase::new();
        db.on_room_detected("room1", "大廳", &[]);
        db.record_last_direction("north");
        // 同一個房間再次偵測（如 look 指令）
        db.on_room_detected("room1", "大廳", &[]);
        assert!(db.moves.is_empty());
    }

    #[test]
    fn test_bfs_find_path() {
        let mut db = MapDatabase::new();
        db.on_room_detected("a", "A", &[]);
        db.record_last_direction("n");
        db.on_room_detected("b", "B", &[]);
        db.record_last_direction("e");
        db.on_room_detected("c", "C", &[]);

        let path = db.find_path("a", "c").unwrap();
        assert_eq!(path, vec!["n", "e"]);
    }

    #[test]
    fn test_find_path_no_route() {
        let mut db = MapDatabase::new();
        db.on_room_detected("a", "A", &[]);
        db.on_room_detected("b", "B", &[]);
        // 沒有邊連接
        assert!(db.find_path("a", "b").is_none());
    }

    #[test]
    fn test_resolve_target() {
        let mut db = MapDatabase::new();
        db.on_room_detected("abc123", "大廳", &[]);
        db.on_room_detected("def456", "走廊", &[]);

        // 精確 ID
        let r = db.resolve_target("abc123");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "abc123");

        // 名稱搜尋
        let r = db.resolve_target("大廳");
        assert_eq!(r.len(), 1);

        // 前綴 ID
        let r = db.resolve_target("abc");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_save_and_load() {
        let mut db = MapDatabase::new();
        db.on_room_detected("room1", "大廳", &["north".into()]);
        db.record_last_direction("north");
        db.on_room_detected("room2", "走廊", &["south".into()]);

        let tmp = std::env::temp_dir().join("test_map_db.json");
        db.save(&tmp).unwrap();

        let loaded = MapDatabase::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.rooms.len(), 2);
        assert_eq!(loaded.moves.get("room1").unwrap().get("n").unwrap(), "room2");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_data_version_increments() {
        let mut db = MapDatabase::new();
        let v0 = db.data_version;
        db.on_room_detected("room1", "大廳", &[]);
        assert!(db.data_version > v0);
    }

    #[test]
    fn test_disabled_does_not_record() {
        let mut db = MapDatabase::new();
        db.disable();
        db.on_room_detected("room1", "大廳", &[]);
        assert!(db.rooms.is_empty());
    }
}
