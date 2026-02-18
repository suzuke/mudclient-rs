use sha2::{Digest, Sha256};
use std::fmt::Write;

/// MUD 房間結構
/// 用於生成唯一 ID
#[derive(Debug, Clone, Default)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub exits: Vec<String>,
}

impl Room {
    /// 建立新房間
    pub fn new(name: &str, description: &str, exits: Vec<String>) -> Self {
        let mut sorted_exits = exits;
        sorted_exits.sort(); // 確保出口順序不影響 ID
        
        Self {
            name: name.to_string(),
            description: description.to_string(),
            exits: sorted_exits,
        }
    }

    /// 生成唯一房間 ID
    ///
    /// # Arguments
    /// * `strict` - 是否包含出口在雜湊計算中
    ///   - `true`: Name + Description + Exits (預設，區分開門狀態)
    ///   - `false`: Name + Description (忽略出口，解決開門後 ID 改變問題，但有碰撞風險)
    pub fn hash(&self, strict: bool) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.description.as_bytes());
        
        if strict {
            for exit in &self.exits {
                hasher.update(exit.as_bytes());
            }
        }
        
        let result = hasher.finalize();
        let mut hex_string = String::new();
        for byte in result {
            write!(&mut hex_string, "{:02x}", byte).expect("Unable to write");
        }
        hex_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_hash_consistency() {
        let room1 = Room::new("Room A", "Description A", vec!["north".to_string(), "south".to_string()]);
        let room2 = Room::new("Room A", "Description A", vec!["south".to_string(), "north".to_string()]); // 出口順序不同

        assert_eq!(room1.hash(true), room2.hash(true), "相同內容但出口順序不同的房間應該有相同的 Hash");
    }

    #[test]
    fn test_room_hash_uniqueness() {
        let room1 = Room::new("Room A", "Description A", vec!["north".to_string()]);
        let room2 = Room::new("Room B", "Description A", vec!["north".to_string()]); // 名稱不同
        let room3 = Room::new("Room A", "Description B", vec!["north".to_string()]); // 描述不同
        let room4 = Room::new("Room A", "Description A", vec!["south".to_string()]); // 出口不同

        assert_ne!(room1.hash(true), room2.hash(true));
        assert_ne!(room1.hash(true), room3.hash(true));
        assert_ne!(room1.hash(true), room4.hash(true));
    }

    #[test]
    fn test_room_hash_lax_mode() {
        let room1 = Room::new("Room A", "Description A", vec!["north".to_string()]);
        let room2 = Room::new("Room A", "Description A", vec!["north".to_string(), "south".to_string()]); // 出口不同 (例如開門)

        // Strict 模式下 ID 不同
        assert_ne!(room1.hash(true), room2.hash(true));

        // Lax 模式下 ID 相同 (忽略出口)
        assert_eq!(room1.hash(false), room2.hash(false));
    }
}
