//\! Profile 管理 UI 渲染
//\!
//\! 包含 Profile 選擇與編輯視窗

use eframe::egui::{self, Color32, RichText};

use super::MudApp;

impl MudApp {
    /// 繪製 Profile 管理視窗 (含連線與新增/編輯/刪除)
    pub(super) fn render_profile_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("👤 連線管理")
            .default_width(400.0)
            .default_height(500.0)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Profile 列表");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ 新增").clicked() {
                            self.editing_profile_original_name = None;
                            self.profile_edit_name = String::new();
                            self.profile_edit_display_name = String::new();
                            self.profile_edit_host = "localhost".to_string();
                            self.profile_edit_port = "7777".to_string();
                            self.profile_edit_username = String::new();
                            self.profile_edit_password = String::new();
                            self.show_profile_edit_window = true;
                        }
                        // 匯入 Profile
                        if ui.button("📥 匯入").clicked() {
                            let import_path = "profiles_export.json";
                            match std::fs::read_to_string(import_path) {
                                Ok(content) => {
                                    if let Ok(profiles) = serde_json::from_str::<Vec<crate::config::Profile>>(&content) {
                                        let mut imported = 0;
                                        for profile in profiles {
                                            if !self.profile_manager.exists(&profile.name) {
                                                if self.profile_manager.save(profile).is_ok() {
                                                    imported += 1;
                                                }
                                            }
                                        }
                                        tracing::info!("匯入了 {} 個 Profile", imported);
                                    }
                                }
                                Err(_) => {
                                    tracing::warn!("找不到 {}", import_path);
                                }
                            }
                        }
                        // 匯出所有 Profile
                        if ui.button("📤 匯出").clicked() {
                            let profiles: Vec<_> = self.profile_manager.list().to_vec();
                            if let Ok(json) = serde_json::to_string_pretty(&profiles) {
                                let export_path = "profiles_export.json";
                                if let Err(e) = std::fs::write(export_path, &json) {
                                    tracing::error!("匯出失敗: {}", e);
                                } else {
                                    tracing::info!("已匯出至 {}", export_path);
                                }
                            }
                        }
                    });
                });
                ui.separator();

                let profiles: Vec<_> = self.profile_manager.list().iter().map(|p| {
                    (p.name.clone(), p.display_name.clone(), p.connection.host.clone(), p.connection.port.clone(), p.username.clone())
                }).collect();

                if profiles.is_empty() {
                    ui.label("尚無任何 Profile。");
                    ui.add_space(10.0);
                } else {
                    egui::ScrollArea::vertical().max_height(ui.available_height() - 80.0).show(ui, |ui| {
                        let mut pending_disconnect_id = None;
                        for (name, display_name, host, port, username) in &profiles {
                            // 檢查連線狀態
                            let connected_session = self.session_manager.sessions().iter().find(|s| {
                                &s.profile_name == name && matches!(s.status, crate::session::ConnectionStatus::Connected(_))
                            });
                            let is_connected = connected_session.is_some();
                            let is_reconnecting = self.session_manager.sessions().iter().any(|s| {
                                &s.profile_name == name && matches!(s.status, crate::session::ConnectionStatus::Reconnecting)
                            });
                            let status_icon = if is_connected { "🟢" } else if is_reconnecting { "🟡" } else { "⚪" };

                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(format!("{} {}", status_icon, display_name)).strong());
                                        let user_info = if let Some(u) = username { format!(" | User: {}", u) } else { String::new() };
                                        ui.label(format!("{}:{}{}", host, port, user_info));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if is_connected {
                                            if ui.button("🔌 斷開").clicked() {
                                                if let Some(s) = connected_session {
                                                    pending_disconnect_id = Some(s.id);
                                                }
                                            }
                                        } else {
                                            if ui.button("🔌 連線").clicked() {
                                                self.pending_connect_profile = Some(name.clone());
                                                self.show_profile_window = false;
                                            }
                                        }
                                        
                                        // 更多操作選單
                                        ui.menu_button("⚙", |ui| {
                                            if ui.button("✏️ 編輯").clicked() {
                                                if let Some(p) = self.profile_manager.get(name) {
                                                    self.editing_profile_original_name = Some(name.clone());
                                                    self.profile_edit_name = p.name.clone();
                                                    self.profile_edit_display_name = p.display_name.clone();
                                                    self.profile_edit_host = p.connection.host.clone();
                                                    self.profile_edit_port = p.connection.port.clone();
                                                    self.profile_edit_username = p.username.clone().unwrap_or_default();
                                                    self.profile_edit_password = p.password.clone().unwrap_or_default();
                                                    self.show_profile_edit_window = true;
                                                }
                                                ui.close_menu();
                                            }
                                            
                                            if ui.button("📋 複製").clicked() {
                                                // 自動產生不重複的名稱
                                                let mut new_name = format!("{}_copy", name);
                                                let mut counter = 2;
                                                while self.profile_manager.exists(&new_name) {
                                                    new_name = format!("{}_copy{}", name, counter);
                                                    counter += 1;
                                                }
                                                if let Err(e) = self.profile_manager.duplicate(name, &new_name) {
                                                    tracing::error!("Failed to duplicate profile: {}", e);
                                                }
                                                ui.close_menu();
                                            }

                                            if ui.button("🗑️ 刪除").clicked() {
                                                if let Err(e) = self.profile_manager.delete(name) {
                                                    tracing::error!("Failed to delete profile: {}", e);
                                                }
                                                ui.close_menu();
                                            }
                                        });
                                    });
                                });
                            });
                        }
                        // 處理斷線請求
                        if let Some(id) = pending_disconnect_id {
                            if let Some(session) = self.session_manager.get_mut(id) {
                                if let Some(tx) = &session.command_tx {
                                    let _ = tx.try_send(crate::session::Command::Disconnect);
                                }
                                session.auto_reconnect = false;
                            }
                        }
                    });
                }

                ui.add_space(15.0);
                ui.separator();

                // 活躍連線列表
                ui.heading("活躍連線");
                ui.separator();

                let session_count = self.session_manager.len();
                if session_count == 0 {
                    ui.label("目前無活躍連線。");
                } else {
                    ui.label(format!("活躍 Session 數量: {}", session_count));
                }

                ui.add_space(15.0);
                if ui.button("關閉").clicked() {
                    self.show_profile_window = false;
                }
            });

        // 渲染 Profile 編輯視窗
        if self.show_profile_edit_window {
            self.render_profile_edit_window(ctx);
        }
    }

    /// 繪製 Profile 編輯視窗
    pub(super) fn render_profile_edit_window(&mut self, ctx: &egui::Context) {
        let title = if self.editing_profile_original_name.is_some() { "✏️ 編輯 Profile" } else { "➕ 新增 Profile" };
        
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("profile_edit_grid_conn").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                    ui.label("識別名稱 (ID):");
                    ui.text_edit_singleline(&mut self.profile_edit_name);
                    ui.end_row();

                    ui.label("顯示名稱:");
                    ui.text_edit_singleline(&mut self.profile_edit_display_name);
                    ui.end_row();

                    ui.label("主機位址 (Host):");
                    ui.text_edit_singleline(&mut self.profile_edit_host);
                    ui.end_row();

                    ui.label("連接埠 (Port):");
                    ui.text_edit_singleline(&mut self.profile_edit_port);
                    ui.end_row();
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                egui::Grid::new("profile_edit_grid_auth").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                    ui.label("帳號 (Username):");
                    ui.text_edit_singleline(&mut self.profile_edit_username);
                    ui.end_row();

                    ui.label("密碼 (Password):");
                    ui.add(egui::TextEdit::singleline(&mut self.profile_edit_password).password(true));
                    ui.end_row();
                });

                if !self.profile_edit_password.is_empty() {
                    ui.colored_label(
                        Color32::from_rgb(100, 200, 100),
                        "🔒 密碼儲存於 OS Keychain"
                    );
                }

                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    if ui.button("💾 儲存").clicked() {
                        // 驗證輸入
                        if self.profile_edit_name.is_empty() {
                            // TODO: 顯示錯誤
                        } else {
                            let mut profile = if let Some(ref original_name) = self.editing_profile_original_name {
                                if let Some(existing) = self.profile_manager.get(original_name) {
                                    existing.clone()
                                } else {
                                    crate::config::Profile::default()
                                }
                            } else {
                                crate::config::Profile::default()
                            };

                            // 更新欄位
                            profile.name = self.profile_edit_name.clone();
                            profile.display_name = self.profile_edit_display_name.clone();
                            profile.connection.host = self.profile_edit_host.clone();
                            profile.connection.port = self.profile_edit_port.clone();
                            
                            profile.username = if self.profile_edit_username.is_empty() { None } else { Some(self.profile_edit_username.clone()) };
                            profile.password = if self.profile_edit_password.is_empty() { None } else { Some(self.profile_edit_password.clone()) };
                            
                            // 如果 ID 被修改，刪除舊 Profile 並更新關聯的 Session
                            if let Some(ref original_name) = self.editing_profile_original_name {
                                if original_name != &profile.name {
                                    // 刪除舊檔案
                                    let _ = self.profile_manager.delete(original_name);
                                    // 更新已連線 Session 的 profile_name
                                    for session in self.session_manager.sessions_mut() {
                                        if session.profile_name == *original_name {
                                            session.profile_name = profile.name.clone();
                                        }
                                    }
                                }
                            }

                            // 儲存
                            if let Err(e) = self.profile_manager.save(profile) {
                                tracing::error!("Failed to save profile: {}", e);
                            }

                            self.show_profile_edit_window = false;
                        }
                    }

                    if ui.button("取消").clicked() {
                        self.show_profile_edit_window = false;
                    }
                });
            });
    }

}
