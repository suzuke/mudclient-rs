//\! 側邊欄 UI 渲染
//\!
//\! 包含工具面板、攻略瀏覽、筆記、地圖等分頁

use eframe::egui;

use super::{MudApp, PendingAction, SidePanelTab};

impl MudApp {
    /// 繪製側邊欄
    pub(super) fn render_side_panel(&mut self, ctx: &egui::Context, active_window_id: String, _active_id: Option<crate::session::SessionId>, pending_action: &mut Option<PendingAction>) {
        egui::SidePanel::right("tools_panel")
            .resizable(true)
            .default_width(250.0) // 增加寬度以容納攻略
            .show(ctx, |ui| {
                // 1. 標籤頁切換
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Tools, "🛠️ 工具");
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Guide, "📖 攻略");
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Notes, "📝 筆記");
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Map, "🗺️ 地圖");
                });
                ui.separator();

                // 2. 內容渲染
                match self.side_panel_tab {
                    SidePanelTab::Tools => {
                        self.render_tools_tab(ui, &active_window_id, pending_action);
                    }
                    SidePanelTab::Guide => {
                        self.render_guide_tab(ui);
                    }
                    SidePanelTab::Notes => {
                        self.render_notes_tab(ui);
                    }
                    SidePanelTab::Map => {
                        self.render_map_tab(ui);
                    }
                }
            });
    }

    /// 繪製工具分頁 (原有的側邊欄內容)
    pub(super) fn render_tools_tab(&mut self, ui: &mut egui::Ui, active_window_id: &str, pending_action: &mut Option<PendingAction>) {
        if let Some(session) = self.session_manager.active_session() {
            ui.heading("視窗");
            ui.separator();

            for window in session.window_manager.windows() {
                let is_active = window.id == active_window_id;
                if ui.selectable_label(is_active, &window.title).clicked() {
                    *pending_action = Some(PendingAction::SwitchWindow(window.id.clone()));
                }
            }

            ui.add_space(15.0);
            ui.heading("管理");
            ui.separator();

            if ui.button("⚙ 設定中心").clicked() {
                *pending_action = Some(PendingAction::ToggleSettings);
            }
            if ui.button("👤 連線管理").clicked() {
                *pending_action = Some(PendingAction::ToggleProfile);
            }
        } else {
            ui.heading("管理");
            ui.separator();
            if ui.button("👤 連線管理").clicked() {
                *pending_action = Some(PendingAction::ToggleProfile);
            }
        }

    }

    /// 繪製攻略分頁
    pub(super) fn render_guide_tab(&mut self, ui: &mut egui::Ui) {
        // 1. 檔案列表區 (上方可摺疊或限制高度)
        ui.group(|ui| {
            ui.label("📚 攻略檔案 (docs/)");
            ui.separator();
            
            // 重新整理按鈕
            if ui.button("🔄 重新整理列表").clicked() || self.guide_file_list.is_empty() {
                self.guide_file_list.clear();
                let docs_dir = std::path::Path::new("docs");
                if docs_dir.exists() {
                     if let Ok(entries) = std::fs::read_dir(docs_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension() {
                                    if ext == "md" || ext == "txt" {
                                        self.guide_file_list.push(path);
                                    }
                                }
                            }
                        }
                        self.guide_file_list.sort();
                    }
                }
            }

            // 檔案列表 Scroll
            ui.push_id("guide_files_scroll", |ui| {
                egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    if self.guide_file_list.is_empty() {
                        ui.label(egui::RichText::new("未找到 .md 或 .txt 檔案").color(egui::Color32::GRAY));
                    } else {
                        for path in &self.guide_file_list {
                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let is_active = self.active_guide_name.as_ref() == Some(&filename);
                            
                            if ui.selectable_label(is_active, &filename).clicked() {
                                self.active_guide_name = Some(filename);
                                if let Ok(content) = std::fs::read_to_string(path) {
                                    self.active_guide_content = content;
                                } else {
                                    self.active_guide_content = "無法讀取檔案內容".to_string();
                                }
                            }
                        }
                    }
                });
            });
        });

        ui.add_space(5.0);
        ui.separator();

        // 2. 內容顯示區
        egui::ScrollArea::vertical()
            .id_salt("guide_content_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.active_guide_content.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("請選擇一個攻略檔案以檢視").color(egui::Color32::GRAY));
                    });
                } else {
                    // 簡易 Markdown 渲染
                    let mut in_code_block = false;
                    for line in self.active_guide_content.lines() {
                        if line.starts_with("```") {
                            in_code_block = !in_code_block;
                            continue;
                        }

                        if in_code_block {
                             // 程式碼區塊樣式
                             ui.label(egui::RichText::new(line).font(egui::FontId::monospace(self.global_config.ui.font_size - 1.0)).color(egui::Color32::LIGHT_GREEN));
                        } else if line.starts_with("# ") {
                            ui.heading(&line[2..]);
                            ui.add_space(5.0);
                        } else if line.starts_with("## ") {
                             ui.label(egui::RichText::new(&line[3..]).heading().size(18.0));
                             ui.add_space(3.0);
                        } else if line.starts_with("### ") {
                             ui.label(egui::RichText::new(&line[4..]).strong().size(16.0));
                        } else if line.starts_with("- ") || line.starts_with("* ") {
                             ui.horizontal(|ui| {
                                 ui.label("•");
                                 ui.label(&line[2..]);
                             });
                        } else {
                            // 普通文字 (支援自動換行)
                            ui.label(line);
                        }
                    }
                }
            });
    }

    /// 繪製筆記分頁
    pub(super) fn render_notes_tab(&mut self, ui: &mut egui::Ui) {
         if let Some(session) = self.session_manager.active_session_mut() {
             ui.label("在此輸入您的個人筆記 (自動儲存)：");
             egui::ScrollArea::vertical().show(ui, |ui| {
                 ui.add(
                     egui::TextEdit::multiline(&mut session.notes)
                         .desired_width(f32::INFINITY)
                         .desired_rows(20)
                         .font(egui::FontId::monospace(self.global_config.ui.font_size)) // 使用等寬字型方便對齊資料
                 );
             });
         } else {
             ui.centered_and_justified(|ui| {
                 ui.label("請先連線以使用筆記功能");
             });
         }
    }

    /// 繪製地圖分頁
    pub(super) fn render_map_tab(&mut self, ui: &mut egui::Ui) {
        // 優先從 active session 的 MapDatabase 即時同步
        if let Some(session) = self.session_manager.active_session() {
            self.map_renderer.sync_from_database(&session.map_database);

            // 同步當前房間
            let current_room_id = session.current_room_id.clone();
            self.map_renderer.set_current_room(current_room_id);
        } else if self.map_renderer.data.is_none() {
            // Fallback: 無 session 時嘗試從檔案載入
            let path = std::path::Path::new("data/mapper_data.json");
            if path.exists() {
                if let Err(e) = self.map_renderer.load_from_file(path) {
                    tracing::warn!("自動載入地圖失敗: {}", e);
                }
            }
        }

        // 渲染地圖，處理動作
        if let Some(action) = self.map_renderer.render(ui) {
            if let Some(session) = self.session_manager.active_session() {
                if let Some(tx) = &session.command_tx {
                    let cmd = match action {
                        crate::mapper::MapAction::StartMapper => "#map start".to_string(),
                        crate::mapper::MapAction::Navigate(target) => {
                            format!("#map go {}", target)
                        }
                    };
                    let _ = tx.try_send(crate::session::Command::Send(cmd));
                }
            }
        }
    }

}
