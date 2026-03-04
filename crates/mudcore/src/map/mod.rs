pub mod database;
pub mod room;

pub use database::MapDatabase;
pub use room::Room;

/// 支援 JSON 中 exits 為 [] 或 {} (空 object) 的情況
/// 供 database.rs 的 RoomRecord 共用
pub(crate) fn room_deserialize_exits<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<String>, D::Error> {
    use serde::de;

    struct ExitsVisitor;
    impl<'de> de::Visitor<'de> for ExitsVisitor {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("array or empty object")
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<String>, A::Error> {
            let mut v = Vec::new();
            while let Some(s) = seq.next_element()? {
                v.push(s);
            }
            Ok(v)
        }
        fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Vec<String>, A::Error> {
            let mut count = 0;
            while map.next_entry::<String, serde_json::Value>()?.is_some() {
                count += 1;
            }
            if count > 0 {
                tracing::warn!("[Map] exits 欄位為非空 object，已忽略 {} 個 entry", count);
            }
            Ok(Vec::new())
        }
    }
    d.deserialize_any(ExitsVisitor)
}
