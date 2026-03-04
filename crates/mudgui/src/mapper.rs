//! MUD 地圖視覺化模組
//!
//! 負責讀取 MudMapper 生成的地圖資料並渲染為可互動的視覺化地圖

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use mudcore::MapDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 地圖資料結構（對應 MudMapper 的 JSON 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapperData {
    /// 房間資料: room_id => RoomInfo
    pub rooms: HashMap<String, RoomInfo>,
    /// 移動連線: from_id => { direction => to_id }
    pub moves: HashMap<String, HashMap<String, String>>,
}

/// 房間資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub visited: u32,
    #[serde(default)]
    pub first_visit: u64,
    #[serde(default, deserialize_with = "deserialize_exits")]
    pub exits: Vec<String>,
}

/// 支援 JSON 中 exits 為 [] 或 {} (空 object) 的情況
fn deserialize_exits<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
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
            // 空 object {} → 空 Vec
            while map.next_entry::<String, serde_json::Value>()?.is_some() {}
            Ok(Vec::new())
        }
    }
    d.deserialize_any(ExitsVisitor)
}

/// 地圖 UI 動作
pub enum MapAction {
    /// 導航到指定房間
    Navigate(String),
    /// 啟動 MudMapper
    StartMapper,
}

/// 地圖渲染器
pub struct MapRenderer {
    /// 地圖資料
    pub data: Option<MapperData>,
    /// 房間佈局快取 (room_id => 畫面座標)
    layout: HashMap<String, Pos2>,
    /// 當前選中的房間 ID
    selected_room: Option<String>,
    /// 當前玩家所在房間 ID
    current_room: Option<String>,
    /// 視圖偏移量
    offset: Vec2,
    /// 縮放比例
    zoom: f32,
    /// 是否需要重新計算佈局
    needs_relayout: bool,
    /// 上次同步的 MapDatabase 版本號
    last_synced_version: u64,
}

impl Default for MapRenderer {
    fn default() -> Self {
        Self {
            data: None,
            layout: HashMap::new(),
            selected_room: None,
            current_room: None,
            offset: Vec2::ZERO,
            zoom: 1.0,
            needs_relayout: true,
            last_synced_version: 0,
        }
    }
}

impl MapRenderer {
    /// 載入地圖資料
    pub fn load_data(&mut self, data: MapperData) {
        self.data = Some(data);
        self.needs_relayout = true;
    }

    /// 從 MapDatabase 即時同步（版本號變更時才觸發 relayout）
    pub fn sync_from_database(&mut self, db: &MapDatabase) {
        if db.data_version == self.last_synced_version && self.data.is_some() {
            return;
        }
        self.last_synced_version = db.data_version;

        let rooms: HashMap<String, RoomInfo> = db.rooms.iter().map(|(id, r)| {
            (id.clone(), RoomInfo {
                id: r.id.clone(),
                name: r.name.clone(),
                visited: r.visited,
                first_visit: r.first_visit,
                exits: r.exits.clone(),
            })
        }).collect();

        let data = MapperData {
            rooms,
            moves: db.moves.clone(),
        };
        self.data = Some(data);
        self.needs_relayout = true;
    }

    /// 從 JSON 檔案載入地圖資料
    pub fn load_from_file(&mut self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("讀取檔案失敗: {}", e))?;
        let data: MapperData = serde_json::from_str(&content)
            .map_err(|e| format!("解析 JSON 失敗: {}", e))?;
        self.load_data(data);
        Ok(())
    }

    /// 設定當前房間
    pub fn set_current_room(&mut self, room_id: Option<String>) {
        if self.current_room != room_id {
            self.current_room = room_id;
            self.needs_relayout = true;
        }
    }

    /// 計算房間佈局（BFS 方向性佈局，含碰撞偏移）
    fn compute_layout(&mut self) {
        self.layout.clear();

        let Some(data) = &self.data else { return };
        if data.rooms.is_empty() { return }

        let start_id = self.current_room.as_ref()
            .or_else(|| data.rooms.keys().next())
            .cloned();
        let Some(start_id) = start_id else { return };

        // 已佔用的格位 (量化後的座標 → room_id)
        let mut occupied = HashMap::<(i32, i32), String>::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        let start_pos = Pos2::new(0.0, 0.0);
        self.layout.insert(start_id.clone(), start_pos);
        occupied.insert(Self::quantize(start_pos), start_id.clone());
        queue.push_back(start_id.clone());
        visited.insert(start_id);

        while let Some(current_id) = queue.pop_front() {
            let current_pos = self.layout[&current_id];

            if let Some(exits) = data.moves.get(&current_id) {
                for (direction, next_id) in exits {
                    if visited.contains(next_id) {
                        continue;
                    }
                    let offset = Self::direction_offset(direction) * 120.0;
                    let ideal_pos = current_pos + offset;
                    let next_pos = Self::resolve_collision(ideal_pos, direction, &occupied);

                    self.layout.insert(next_id.clone(), next_pos);
                    occupied.insert(Self::quantize(next_pos), next_id.clone());
                    queue.push_back(next_id.clone());
                    visited.insert(next_id.clone());
                }
            }
        }

        self.needs_relayout = false;
    }

    /// 將座標量化為格位鍵（用於碰撞檢測）
    fn quantize(pos: Pos2) -> (i32, i32) {
        ((pos.x / 60.0).round() as i32, (pos.y / 60.0).round() as i32)
    }

    /// 碰撞偏移：若理想位置已佔用，沿方向的垂直軸偏移
    fn resolve_collision(
        ideal: Pos2,
        direction: &str,
        occupied: &HashMap<(i32, i32), String>,
    ) -> Pos2 {
        let key = Self::quantize(ideal);
        if !occupied.contains_key(&key) {
            return ideal;
        }

        // 沿移動方向的垂直軸偏移
        let perp = match direction.to_lowercase().as_str() {
            "n" | "north" | "s" | "south" => Vec2::new(1.0, 0.0),
            "e" | "east" | "w" | "west" => Vec2::new(0.0, 1.0),
            _ => Vec2::new(1.0, 1.0),
        };

        for i in 1..=10 {
            for sign in [1.0_f32, -1.0] {
                let candidate = ideal + perp * (60.0 * i as f32 * sign);
                if !occupied.contains_key(&Self::quantize(candidate)) {
                    return candidate;
                }
            }
        }
        ideal
    }

    /// 根據方向字串返回單位向量
    fn direction_offset(dir: &str) -> Vec2 {
        match dir.to_lowercase().as_str() {
            "n" | "north" => Vec2::new(0.0, -1.0),
            "s" | "south" => Vec2::new(0.0, 1.0),
            "e" | "east" => Vec2::new(1.0, 0.0),
            "w" | "west" => Vec2::new(-1.0, 0.0),
            "ne" | "northeast" => Vec2::new(0.7, -0.7),
            "nw" | "northwest" => Vec2::new(-0.7, -0.7),
            "se" | "southeast" => Vec2::new(0.7, 0.7),
            "sw" | "southwest" => Vec2::new(-0.7, 0.7),
            "u" | "up" => Vec2::new(0.0, -0.5),
            "d" | "down" => Vec2::new(0.0, 0.5),
            _ => Vec2::ZERO,
        }
    }

    /// 渲染地圖回傳動作：導航目標 room_id 或啟動 mapper 指令
    pub fn render(&mut self, ui: &mut egui::Ui) -> Option<MapAction> {
        if self.data.is_none() {
            let mut action = None;
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("尚未載入地圖資料");
                ui.add_space(10.0);
                ui.label(egui::RichText::new("地圖記錄預設啟用，移動後將自動顯示。").small().color(Color32::GRAY));
                ui.add_space(5.0);
                if ui.button("啟動 #map start").clicked() {
                    action = Some(MapAction::StartMapper);
                }
            });
            return action;
        }

        if self.needs_relayout {
            self.compute_layout();
        }

        // 控制面板
        ui.horizontal(|ui| {
            if ui.button("重新載入").clicked() {
                let path = Path::new("data/mapper_data.json");
                if let Err(e) = self.load_from_file(path) {
                    tracing::error!("載入地圖失敗: {}", e);
                }
            }
            if ui.button("重置視圖").clicked() {
                self.offset = Vec2::ZERO;
                self.zoom = 1.0;
            }
            ui.label(format!("房間: {}", self.layout.len()));
        });

        ui.separator();

        // 繪製區域
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            Sense::click_and_drag(),
        );

        // 拖曳平移
        if response.dragged() {
            self.offset += response.drag_delta();
        }

        // 滾輪縮放
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.003)).clamp(0.2, 5.0);
            }
        }

        let to_screen = |pos: Pos2| -> Pos2 {
            let center = response.rect.center();
            center + (pos.to_vec2() * self.zoom) + self.offset
        };

        let Some(data) = &self.data else { return None };

        // 裁剪矩形：只繪製畫布可見區域（預留 margin）
        let cull_rect = response.rect.expand(80.0);

        // 繪製連線
        for (from_id, exits) in &data.moves {
            let Some(&from_pos) = self.layout.get(from_id) else { continue };
            for to_id in exits.values() {
                let Some(&to_pos) = self.layout.get(to_id) else { continue };
                let from_screen = to_screen(from_pos);
                let to_screen_pos = to_screen(to_pos);
                if !cull_rect.contains(from_screen) && !cull_rect.contains(to_screen_pos) {
                    continue;
                }
                painter.line_segment(
                    [from_screen, to_screen_pos],
                    Stroke::new(1.5, Color32::from_gray(100)),
                );
            }
        }

        // 繪製房間節點
        for (room_id, pos) in &self.layout {
            let screen_pos = to_screen(*pos);
            if !cull_rect.contains(screen_pos) {
                continue;
            }
            let radius = 25.0 * self.zoom;
            let Some(room_info) = data.rooms.get(room_id) else { continue };

            let is_current = Some(room_id.as_str()) == self.current_room.as_deref();
            let is_selected = Some(room_id.as_str()) == self.selected_room.as_deref();

            let (fill, stroke_color, sw) = if is_current {
                (Color32::from_rgb(100, 200, 100), Color32::from_rgb(50, 255, 50), 3.0)
            } else if is_selected {
                (Color32::from_rgb(200, 200, 100), Color32::from_rgb(255, 255, 50), 2.5)
            } else {
                (Color32::from_rgb(80, 80, 120), Color32::from_rgb(150, 150, 180), 1.5)
            };

            // 根據房間名稱長度動態調整圓圈大小
            let font_size = 11.0 * self.zoom;
            let text_width = room_info.name.chars().count() as f32 * font_size * 0.55;
            let min_radius = radius;
            let actual_radius = min_radius.max(text_width / 2.0 + 8.0 * self.zoom);

            painter.circle(screen_pos, actual_radius, fill, Stroke::new(sw, stroke_color));

            // 房間名稱（置中在圓圈內）
            painter.text(
                screen_pos,
                egui::Align2::CENTER_CENTER,
                &room_info.name,
                egui::FontId::proportional(font_size),
                Color32::WHITE,
            );

            // 點擊選取
            let rect = Rect::from_center_size(screen_pos, Vec2::splat(actual_radius * 2.0));
            let room_resp = ui.interact(rect, ui.id().with(room_id), Sense::click());
            if room_resp.clicked() {
                self.selected_room = Some(room_id.clone());
            }

            // Tooltip
            room_resp.on_hover_ui(|ui| {
                let id_short = if room_id.len() > 12 { &room_id[..12] } else { room_id };
                ui.label(&room_info.name);
                ui.label(format!("ID: {}...", id_short));
                ui.label(format!("訪問: {} 次", room_info.visited));
            });
        }

        // 選中房間詳情
        let mut action = None;
        if let Some(selected_id) = &self.selected_room.clone() {
            if let Some(room_info) = data.rooms.get(selected_id) {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("{}", room_info.name));
                    if ui.button("前往").clicked() {
                        action = Some(MapAction::Navigate(selected_id.clone()));
                    }
                });
            }
        }

        action
    }
}
