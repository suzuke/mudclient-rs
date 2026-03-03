//\! 設定中心 UI 渲染
//\!
//\! 包含別名編輯、觸發器編輯、路徑編輯以及設定視窗

use eframe::egui::{self, Color32, RichText, TextEdit};
use egui_extras::{Column, TableBuilder};
use mudcore::{Alias, Trigger, TriggerAction, TriggerPattern, Path};

use crate::config::GlobalConfig;

use super::{MudApp, SettingsTab, SettingsScope, clean_pattern_string, chrono_lite_timestamp};

impl MudApp {
    /// 繪製別名編輯介面
    pub(super) fn render_alias_edit(
        ctx: &egui::Context,
        session_opt: Option<&mut crate::session::Session>,
        global_config_opt: Option<&mut GlobalConfig>,
        editing_alias_name: &mut Option<String>,
        alias_edit_name: &mut String,
        alias_edit_pattern: &mut String,
        alias_edit_replacement: &mut String,
        alias_edit_category: &mut String,
        alias_edit_is_script: &mut bool,
        show_alias_window: &mut bool,
        needs_save_flag: &mut bool,
    ) {
        egui::Window::new(if editing_alias_name.as_ref().map_or(true, |n| n.is_empty()) { "➕ 新增別名" } else { "✏️ 編輯別名" })
            .collapsible(false)
            .resizable(true)
            .default_width(450.0)
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("名稱:");
                    ui.add(TextEdit::singleline(alias_edit_name).desired_width(f32::INFINITY));
                });

                ui.horizontal(|ui| {
                    ui.label("觸發詞:");
                    ui.add(TextEdit::singleline(alias_edit_pattern).desired_width(f32::INFINITY));
                });

                ui.horizontal(|ui| {
                    ui.checkbox(alias_edit_is_script, "使用 Lua 腳本");
                    ui.label(
                        egui::RichText::new("(勾選後可撰寫多行程式碼)")
                            .size(11.0)
                            .color(egui::Color32::GRAY)
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(if *alias_edit_is_script { "Lua 腳本:" } else { "替換為:" });
                    if *alias_edit_is_script {
                        ui.add(TextEdit::multiline(alias_edit_replacement).desired_rows(8).desired_width(f32::INFINITY));
                    } else {
                        ui.add(TextEdit::singleline(alias_edit_replacement).desired_width(f32::INFINITY));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("分類:");
                    ui.text_edit_singleline(alias_edit_category);

                    // 分類選擇選單
                    ui.menu_button("▼", |ui| {
                        ui.set_max_width(200.0);
                        
                        // 收集並排序現有的所有分類
                        let mut categories: Vec<String> = Vec::new();

                        if let Some(session) = session_opt.as_ref() {
                            categories.extend(session.trigger_manager.list().iter().filter_map(|t| t.category.clone()));
                            categories.extend(session.alias_manager.list().iter().filter_map(|a| a.category.clone()));
                        } else if let Some(global) = global_config_opt.as_ref() {
                             categories.extend(global.global_triggers.iter().filter_map(|t| t.category.clone()));
                             categories.extend(global.global_aliases.iter().filter_map(|a| a.category.clone()));
                        }
                        
                        categories.retain(|c| !c.is_empty());
                        categories.sort();
                        categories.dedup();

                        if categories.is_empty() {
                            ui.label("尚無任何分類");
                        } else {
                            ui.label("選擇現有分類:");
                            ui.separator();
                            for cat in categories {
                                if ui.button(&cat).clicked() {
                                    *alias_edit_category = cat;
                                    ui.close_menu();
                                }
                            }
                        }
                    });
                });

                ui.add_space(10.0);
                ui.label("提示: 使用 $1, $2 等作為參數佔位符");

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("💾 儲存").clicked() {
                        if !alias_edit_pattern.is_empty() {
                            // 名稱空白時 fallback 用 pattern
                            let final_name = if alias_edit_name.is_empty() {
                                alias_edit_pattern.clone()
                            } else {
                                alias_edit_name.clone()
                            };

                            if let Some(session) = session_opt {
                                // 如果是編輯模式，先刪除舊的
                                if let Some(ref old_name) = editing_alias_name {
                                    if !old_name.is_empty() {
                                        session.alias_manager.remove(old_name);
                                    }
                                }
                                // 新增別名
                                let mut alias = Alias::new(
                                    final_name,
                                    alias_edit_pattern.clone(),
                                    alias_edit_replacement.clone(),
                                );
                                alias.is_script = *alias_edit_is_script;
                                if !alias_edit_category.is_empty() {
                                    alias.category = Some(alias_edit_category.clone());
                                }
                                session.alias_manager.add(alias);
                                *needs_save_flag = true;
                            } else if let Some(global) = global_config_opt {
                                // Global Config Logic
                                if let Some(ref old_name) = editing_alias_name {
                                    if !old_name.is_empty() {
                                        global.global_aliases.retain(|a| &a.name != old_name);
                                    }
                                }

                                // Push new
                                global.global_aliases.push(crate::config::AliasConfig {
                                    name: final_name,
                                    pattern: alias_edit_pattern.clone(),
                                    replacement: alias_edit_replacement.clone(),
                                    category: if alias_edit_category.is_empty() { None } else { Some(alias_edit_category.clone()) },
                                    is_script: *alias_edit_is_script,
                                    enabled: true,
                                });
                                *needs_save_flag = true;
                            }
                            *show_alias_window = false;
                        }
                    }

                    if ui.button("取消").clicked() {
                        *show_alias_window = false;
                    }
                });
            });
    }

    /// 繪製觸發器編輯介面
    pub(super) fn render_trigger_edit(
        ctx: &egui::Context,
        session_opt: Option<&mut crate::session::Session>,
        global_config_opt: Option<&mut GlobalConfig>,
        editing_trigger_name: &mut Option<String>,
        trigger_edit_name: &mut String,
        trigger_edit_pattern: &mut String,
        trigger_edit_action: &mut String,
        trigger_edit_category: &mut String,
        trigger_edit_is_script: &mut bool,
        trigger_edit_pattern_type: &mut String,
        show_trigger_window: &mut bool,
        needs_save_flag: &mut bool,
    ) {
        egui::Window::new(if editing_trigger_name.as_ref().map_or(true, |n| n.is_empty()) { "➕ 新增觸發器" } else { "✏️ 編輯觸發器" })
            .collapsible(false)
            .resizable(true)
            .default_width(450.0)
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("名稱:");
                    ui.add(TextEdit::singleline(trigger_edit_name).desired_width(f32::INFINITY));
                });

                ui.horizontal(|ui| {
                    ui.label("匹配文字:");
                    ui.add(TextEdit::singleline(trigger_edit_pattern).desired_width(f32::INFINITY));
                });

                // 匹配類型選擇
                ui.horizontal(|ui| {
                    ui.label("匹配類型:");
                    egui::ComboBox::from_id_salt("trigger_pattern_type")
                        .selected_text(match trigger_edit_pattern_type.as_str() {
                            "contains" => "包含 (Contains)",
                            "startswith" => "開頭 (StartsWith)",
                            "endswith" => "結尾 (EndsWith)",
                            "regex" => "正則 (Regex)",
                            _ => "自動偵測 (Auto)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(trigger_edit_pattern_type, "auto".to_string(), "自動偵測 (Auto)");
                            ui.selectable_value(trigger_edit_pattern_type, "contains".to_string(), "包含 (Contains)");
                            ui.selectable_value(trigger_edit_pattern_type, "startswith".to_string(), "開頭 (StartsWith)");
                            ui.selectable_value(trigger_edit_pattern_type, "endswith".to_string(), "結尾 (EndsWith)");
                            ui.selectable_value(trigger_edit_pattern_type, "regex".to_string(), "正則 (Regex)");
                        });
                });

                ui.add_space(5.0);

                // 1. Lua 選項上移
                ui.horizontal(|ui| {
                    ui.checkbox(trigger_edit_is_script, "使用 Lua 腳本模式");
                    ui.label(
                        egui::RichText::new("(勾選後可撰寫多行程式碼)")
                            .size(11.0)
                            .color(egui::Color32::GRAY)
                    );
                });

                // 2. 執行命令 (根據模式切換單行/多行)
                ui.horizontal(|ui| {
                    ui.label("執行內容:");
                    if *trigger_edit_is_script {
                        ui.add(TextEdit::multiline(trigger_edit_action).desired_rows(8).desired_width(f32::INFINITY));
                    } else {
                        ui.add(TextEdit::singleline(trigger_edit_action).desired_width(f32::INFINITY));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("分類標籤:");
                    ui.text_edit_singleline(trigger_edit_category);

                    // 分類選擇選單
                    ui.menu_button("▼", |ui| {
                        ui.set_max_width(200.0);
                        
                        // 收集並排序現有的所有分類
                        let mut categories: Vec<String> = Vec::new();
                        
                        if let Some(session) = session_opt.as_ref() {
                            categories.extend(session.trigger_manager.list().iter().filter_map(|t| t.category.clone()));
                            categories.extend(session.alias_manager.list().iter().filter_map(|a| a.category.clone()));
                        } else if let Some(global) = global_config_opt.as_ref() {
                             categories.extend(global.global_triggers.iter().filter_map(|t| t.category.clone()));
                             categories.extend(global.global_aliases.iter().filter_map(|a| a.category.clone()));
                        }
                        
                        categories.retain(|c| !c.is_empty());
                        categories.sort();
                        categories.dedup();

                        if categories.is_empty() {
                            ui.label("尚無任何分類");
                        } else {
                            ui.label("選擇現有分類:");
                            ui.separator();
                            for cat in categories {
                                if ui.button(&cat).clicked() {
                                    *trigger_edit_category = cat;
                                    ui.close_menu();
                                }
                            }
                        }
                    });
                });

                ui.add_space(10.0);
                // 3. 優化提示文字
                ui.label(
                    egui::RichText::new("💡 小撇步：匹配文字支援 Regular Expression (正則表達式)，讓您的觸發器更聰明！")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(180, 180, 180))
                );

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("💾 儲存").clicked() {
                        if !trigger_edit_name.is_empty() && !trigger_edit_pattern.is_empty() {
                            // 根據匹配類型建立 Pattern
                            let pattern = match trigger_edit_pattern_type.as_str() {
                                "contains" => TriggerPattern::Contains(trigger_edit_pattern.clone()),
                                "startswith" => TriggerPattern::StartsWith(trigger_edit_pattern.clone()),
                                "endswith" => TriggerPattern::EndsWith(trigger_edit_pattern.clone()),
                                "regex" => TriggerPattern::Regex(trigger_edit_pattern.clone()),
                                _ => {
                                    // auto: 自動偵測
                                    if trigger_edit_pattern.contains("(.+)")
                                        || trigger_edit_pattern.contains("(.*)")
                                        || trigger_edit_pattern.contains("\\d")
                                        || trigger_edit_pattern.contains("[")
                                        || trigger_edit_pattern.contains("$")
                                        || trigger_edit_pattern.contains("^")
                                        || trigger_edit_pattern.contains("|")
                                        || trigger_edit_pattern.contains("?")
                                    {
                                        TriggerPattern::Regex(trigger_edit_pattern.clone())
                                    } else {
                                        TriggerPattern::Contains(trigger_edit_pattern.clone())
                                    }
                                }
                            };
                            let config_pattern_type = if trigger_edit_pattern_type == "auto" {
                                None
                            } else {
                                Some(trigger_edit_pattern_type.clone())
                            };

                            if let Some(session) = session_opt {
                                // 如果是編輯模式，先刪除舊的
                                if let Some(ref old_name) = editing_trigger_name {
                                    if !old_name.is_empty() {
                                        session.trigger_manager.remove(old_name);
                                    }
                                }
                                let mut trigger = Trigger::new(
                                    trigger_edit_name.clone(),
                                    pattern,
                                );
                                if !trigger_edit_action.is_empty() {
                                    if *trigger_edit_is_script {
                                        trigger = trigger.add_action(TriggerAction::ExecuteScript(trigger_edit_action.clone()));
                                    } else {
                                        trigger = trigger.add_action(TriggerAction::SendCommand(trigger_edit_action.clone()));
                                    }
                                }
                                if !trigger_edit_category.is_empty() {
                                    trigger.category = Some(trigger_edit_category.clone());
                                }
                                session.trigger_manager.add(trigger);
                                *needs_save_flag = true;
                            } else if let Some(global) = global_config_opt {
                                // Global Config Logic
                                if let Some(ref old_name) = editing_trigger_name {
                                    if !old_name.is_empty() {
                                        global.global_triggers.retain(|t| &t.name != old_name);
                                    }
                                }

                                global.global_triggers.push(crate::config::TriggerConfig {
                                    name: trigger_edit_name.clone(),
                                    pattern: trigger_edit_pattern.clone(),
                                    action: trigger_edit_action.clone(),
                                    category: if trigger_edit_category.is_empty() { None } else { Some(trigger_edit_category.clone()) },
                                    is_script: *trigger_edit_is_script,
                                    enabled: true,
                                    pattern_type: config_pattern_type,
                                });
                                *needs_save_flag = true;
                            }
                            *show_trigger_window = false;
                        }
                    }

                    if ui.button("取消").clicked() {
                        *show_trigger_window = false;
                    }
                });
            });
    }

    /// 繪製路徑編輯介面
    pub(super) fn render_path_edit(
        ctx: &egui::Context,
        session_opt: Option<&mut crate::session::Session>,
        editing_path_name: &mut Option<String>,
        path_edit_name: &mut String,
        path_edit_value: &mut String,
        path_edit_category: &mut String,
        show_path_window: &mut bool,
        needs_save_flag: &mut bool,
    ) {
        egui::Window::new(if editing_path_name.as_ref().map_or(true, |n| n.is_empty()) { "➕ 新增路徑" } else { "✏️ 編輯路徑" })
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("路徑名稱:");
                    ui.text_edit_singleline(path_edit_name);
                });

                ui.horizontal(|ui| {
                    ui.label("路徑內容:");
                    ui.text_edit_singleline(path_edit_value);
                });
                
                ui.label(
                    egui::RichText::new("提示: 使用 /3w2ne 格式可自動解析為 recall; w; w; w; ne; ne")
                        .size(11.0)
                        .color(egui::Color32::GRAY)
                );

                ui.horizontal(|ui| {
                    ui.label("分類:");
                    ui.text_edit_singleline(path_edit_category);

                    // 分類選擇選單
                    if let Some(session) = session_opt.as_ref() {
                        ui.menu_button("▼", |ui| {
                            ui.set_max_width(200.0);
                            
                            // 收集現有分類
                            let mut categories: Vec<String> = Vec::new();
                            categories.extend(session.path_manager.list().iter().filter_map(|p| p.category.clone()));
                            
                            categories.retain(|c| !c.is_empty());
                            categories.sort();
                            categories.dedup();

                            if categories.is_empty() {
                                ui.label("尚無任何分類");
                            } else {
                                ui.label("選擇現有分類:");
                                ui.separator();
                                for cat in categories {
                                    if ui.button(&cat).clicked() {
                                        *path_edit_category = cat;
                                        ui.close_menu();
                                    }
                                }
                            }
                        });
                    }
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("💾 儲存").clicked() {
                        if !path_edit_name.is_empty() && !path_edit_value.is_empty() {
                            if let Some(session) = session_opt {
                                // 如果是編輯模式，先刪除舊的
                                if let Some(ref old_name) = editing_path_name {
                                    if !old_name.is_empty() {
                                        session.path_manager.remove(old_name);
                                    }
                                }
                                // 新增路徑
                                let mut path = Path::new(
                                    path_edit_name.clone(),
                                    path_edit_value.clone(),
                                );
                                if !path_edit_category.is_empty() {
                                    path.category = Some(path_edit_category.clone());
                                }
                                session.path_manager.add(path);
                                *needs_save_flag = true;
                            }
                            *show_path_window = false;
                        }
                    }

                    if ui.button("取消").clicked() {
                        *show_path_window = false;
                    }
                });
            });
    }

    /// 繪製設定視窗 (獨立 Window)
    pub(super) fn render_settings_window(&mut self, ctx: &egui::Context) {
        let mut should_close = false;
        let mut needs_save = false;
        
        egui::Window::new("⚙ 設定中心")
            .resizable(true)
            .default_width(550.0)
            .default_height(450.0)
            .collapsible(false)
            .show(ctx, |ui| {
                // 獲取活躍 session
                let session = match self.session_manager.active_session_mut() {
                    Some(s) => s,
                    None => {
                        ui.label("請先連線至 MUD 伺服器。");
                        ui.add_space(10.0);
                        if ui.button("關閉").clicked() {
                            should_close = true;
                        }
                        return;
                    }
                };

                // Tab 選擇
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Alias, "別名 (Alias)");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Trigger, "觸發器 (Trigger)");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Path, "路徑 (Path)");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Logger, "日誌 (Logger)");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::General, "一般 (General)");
                });
                ui.separator();
                
                // 設定範圍選擇 (僅對 Alias 與 Trigger 有效)
                if matches!(self.settings_tab, SettingsTab::Alias | SettingsTab::Trigger) {
                    ui.horizontal(|ui| {
                        ui.label("設定範圍:");
                        ui.radio_value(&mut self.settings_scope, SettingsScope::Profile, "目前 Profile");
                        ui.radio_value(&mut self.settings_scope, SettingsScope::Global, "全域設定 (Global)");
                    });
                    if self.settings_scope == SettingsScope::Global {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "ℹ️ 正在編輯全域設定，所有 Profile 預設都會套用這些設定。");
                    }
                    ui.separator();
                }

                // 根據目前的 Tab 渲染內容
                match self.settings_tab {
                    SettingsTab::Alias => {
                        ui.horizontal(|ui| {
                            ui.heading(match self.settings_scope {
                                SettingsScope::Profile => "別名管理 (Profile)",
                                SettingsScope::Global => "別名管理 (Global)",
                            });

                            // 搜尋框
                            ui.add(TextEdit::singleline(&mut self.alias_search_text).hint_text("🔍 搜尋名稱或內容..."));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("➕ 新增別名").clicked() {
                                    self.editing_alias_name = Some(String::new());
                                    self.alias_edit_name = String::new();
                                    self.alias_edit_pattern = String::new();
                                    self.alias_edit_replacement = String::new();
                                    self.alias_edit_category = String::new();
                                    self.alias_edit_is_script = false;
                                    self.show_alias_window = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        
                        // 定義別名來源類型
                        #[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
                        enum AliasSource {
                            Global,   // 來自全域設定 (繼承)
                            Profile,  // 本地設定 (獨有)
                            Override, // 本地設定 (覆蓋全域)
                        }

                        // 收集 Alias 列表
                        let mut alias_list: Vec<(String, String, String, Option<String>, bool, bool, AliasSource)> = match self.settings_scope {
                            SettingsScope::Profile => {
                                // Profile 模式: 顯示 Session 中的別名
                                session.alias_manager.sorted_aliases.iter()
                                    .filter_map(|name| {
                                        session.alias_manager.aliases.get(name).map(|a| {
                                            // 判斷來源
                                            let source = if let Some(global_a) = self.global_config.global_aliases.iter().find(|ga| ga.name == a.name) {
                                                let global_is_match = global_a.pattern == a.pattern &&
                                                                    global_a.replacement == a.replacement &&
                                                                    global_a.is_script == a.is_script &&
                                                                    global_a.enabled == a.enabled &&
                                                                    global_a.category == a.category;
                                                                    
                                                if global_is_match {
                                                    AliasSource::Global
                                                } else {
                                                    AliasSource::Override
                                                }
                                            } else {
                                                AliasSource::Profile
                                            };

                                            (a.name.clone(), a.pattern.clone(), a.replacement.clone(), a.category.clone(), a.enabled, a.is_script, source)
                                        })
                                    })
                                    .collect()
                            },
                            SettingsScope::Global => {
                                // Global 模式: 顯示 Global Config 中的別名
                                self.global_config.global_aliases.iter().map(|a| {
                                    (a.name.clone(), a.pattern.clone(), a.replacement.clone(), a.category.clone(), a.enabled, a.is_script, AliasSource::Global)
                                }).collect()
                            }
                        };
                        
                        // 搜尋過濾
                        let search = self.alias_search_text.to_lowercase();
                        if !search.is_empty() {
                            alias_list.retain(|(name, pattern, replacement, cat, _, _, _)| {
                                name.to_lowercase().contains(&search) || 
                                pattern.to_lowercase().contains(&search) ||
                                replacement.to_lowercase().contains(&search) ||
                                cat.as_deref().unwrap_or("").to_lowercase().contains(&search)
                            });
                        }

                        // Grouping Logic
                        let mut grouped_aliases: std::collections::BTreeMap<Option<String>, Vec<(String, String, String, Option<String>, bool, bool, AliasSource)>> = std::collections::BTreeMap::new();
                        for item in alias_list {
                            grouped_aliases.entry(item.3.clone()).or_default().push(item);
                        }

                        let mut to_delete: Option<String> = None;
                        let mut to_edit: Option<(String, String, String, String, bool)> = None;
                        let mut to_toggle_name: Option<(String, bool)> = None;
                        let mut to_toggle_category: Option<(Option<String>, bool)> = None;

                        // 操作 Action
                        enum AliasOp {
                            MoveToGlobal(String),
                            MoveToProfile(String),
                            RevertToGlobal(String),
                            CopyToGlobal(String),
                            Clone(String),
                        }
                        let mut op_action: Option<AliasOp> = None;
                        let mut set_pending_delete: Option<Option<String>> = None;

                        // 表格繪製
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::auto()) // Enabled
                            .column(Column::auto()) // Source Icon
                            .column(Column::initial(120.0).resizable(true)) // Name
                            .column(Column::initial(180.0).resizable(true)) // Pattern
                            .column(Column::remainder()) // Replacement
                            .column(Column::auto()) // Actions
                            .header(20.0, |mut header| {
                                header.col(|ui| { ui.strong("啟用"); });
                                header.col(|ui| { ui.strong("來源"); });
                                header.col(|ui| { ui.strong("名稱"); });
                                header.col(|ui| { ui.strong("指令"); });
                                header.col(|ui| { ui.strong("內容"); });
                                header.col(|ui| { ui.strong("操作"); });
                            })
                            .body(|mut body| {
                                for (category, items) in grouped_aliases {
                                     let category_id_str = category.clone().unwrap_or_else(|| "default".to_string());
                                     let is_expanded_id = body.ui_mut().make_persistent_id(format!("alias_cat_{}", category_id_str));
                                     let is_expanded = body.ui_mut().data(|d| d.get_temp::<bool>(is_expanded_id).unwrap_or(false));

                                     // Group Header Row
                                     body.row(24.0, |mut row| {
                                         row.col(|ui| {
                                             let icon = if is_expanded { "▼" } else { "▶" };
                                             if ui.button(icon).clicked() {
                                                 ui.data_mut(|d| d.insert_temp(is_expanded_id, !is_expanded));
                                             }
                                         });
                                         row.col(|_| {}); // Source placeholder
                                         row.col(|ui| {
                                             let cat_name = category.as_deref().unwrap_or("未分類");
                                             ui.strong(cat_name);
                                         });
                                         row.col(|ui| {
                                             // Batch toggle
                                            if !items.is_empty() {
                                                let all_enabled = items.iter().all(|i| i.4);
                                                let mut current_all = all_enabled;
                                                if ui.checkbox(&mut current_all, "(全選)").changed() {
                                                    to_toggle_category = Some((category.clone(), current_all));
                                                }
                                            }
                                         });
                                         row.col(|_| {});
                                         row.col(|_| {});
                                     });

                                    if is_expanded {
                                        for (name, pattern, replacement, cat, enabled, is_script, source) in items {
                                            body.row(24.0, |mut row| {
                                                // 停用項目文字變暗
                                                let text_color = if enabled {
                                                    Color32::WHITE
                                                } else {
                                                    Color32::from_gray(100)
                                                };

                                                // 1. 啟用
                                                row.col(|ui| {
                                                    let mut is_enabled = enabled;
                                                    if ui.checkbox(&mut is_enabled, "").changed() {
                                                        to_toggle_name = Some((name.clone(), is_enabled));
                                                    }
                                                });

                                                // 2. 來源圖示
                                                row.col(|ui| {
                                                    match source {
                                                        AliasSource::Global => { ui.label("🌍").on_hover_text("全域設定 (Global)"); },
                                                        AliasSource::Profile => { ui.label("👤").on_hover_text("Profile 專屬"); },
                                                        AliasSource::Override => { ui.label("⚠️").on_hover_text("已覆蓋全域設定 (Override)"); },
                                                    }
                                                });

                                                // 3. 名稱
                                                row.col(|ui| {
                                                    let rt = RichText::new(&name).color(text_color);
                                                    ui.label(rt);
                                                });

                                                // 4. 指令 (Pattern)
                                                row.col(|ui| {
                                                    let rt = RichText::new(&pattern).color(text_color);
                                                    ui.label(rt).on_hover_text(&pattern);
                                                });

                                                // 5. 內容 (Replacement)
                                                row.col(|ui| {
                                                    let display_text = if is_script {
                                                        let first_line = replacement.lines().next().unwrap_or("");
                                                        let truncated = if first_line.chars().count() > 40 {
                                                            format!("{}...", first_line.chars().take(40).collect::<String>())
                                                        } else {
                                                            first_line.to_string()
                                                        };
                                                        format!("[Lua] {}", truncated)
                                                    } else {
                                                        replacement.clone()
                                                    };
                                                    let mut rt = RichText::new(&display_text).color(text_color);
                                                    if is_script { rt = rt.italics(); }
                                                    ui.label(rt).on_hover_text(&replacement);
                                                });

                                                // 6. 操作
                                                row.col(|ui| {
                                                     ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 8.0;
                                                        if ui.button("✏️").on_hover_text("編輯").clicked() {
                                                            to_edit = Some((name.clone(), pattern.clone(), replacement.clone(), cat.clone().unwrap_or_default(), is_script));
                                                        }

                                                        if self.settings_scope == SettingsScope::Profile {
                                                            ui.menu_button(" ⋮ ", |ui| {
                                                                ui.set_min_width(120.0);
                                                                if ui.button("📋 複製").clicked() {
                                                                    op_action = Some(AliasOp::Clone(name.clone()));
                                                                    ui.close_menu();
                                                                }
                                                                ui.separator();
                                                                match source {
                                                                    AliasSource::Profile => {
                                                                        if ui.button("🌍 移至全域").clicked() {
                                                                            op_action = Some(AliasOp::MoveToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                        if ui.button("📋 複製至全域").clicked() {
                                                                            op_action = Some(AliasOp::CopyToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                    },
                                                                    AliasSource::Global => {
                                                                        if ui.button("👤 獨立為 Profile").clicked() {
                                                                            op_action = Some(AliasOp::MoveToProfile(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                        if ui.button("✏️ 覆蓋 (Override)").clicked() {
                                                                            to_edit = Some((name.clone(), pattern.clone(), replacement.clone(), cat.clone().unwrap_or_default(), is_script));
                                                                            ui.close_menu();
                                                                        }
                                                                    },
                                                                    AliasSource::Override => {
                                                                        if ui.button("🔙 還原至全域").clicked() {
                                                                            op_action = Some(AliasOp::RevertToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                        if ui.button("🌍 更新至全域").clicked() {
                                                                            op_action = Some(AliasOp::MoveToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                        }

                                                        // 兩段式刪除確認
                                                        if self.pending_alias_delete.as_deref() == Some(&name) {
                                                            if ui.button(RichText::new("確認").color(Color32::RED)).clicked() {
                                                                to_delete = Some(name.clone());
                                                                set_pending_delete = Some(None);
                                                            }
                                                            if ui.button(RichText::new("取消").color(Color32::GRAY)).clicked() {
                                                                set_pending_delete = Some(None);
                                                            }
                                                        } else if ui.button("🗑️").on_hover_text("刪除").clicked() {
                                                            set_pending_delete = Some(Some(name.clone()));
                                                        }
                                                     });
                                                });
                                            });
                                        }
                                    }
                                }
                            });

                        // Apply pending delete state
                        if let Some(new_val) = set_pending_delete {
                            self.pending_alias_delete = new_val;
                        }
                        
                        // 處理操作
                        if let Some((cat, enabled)) = to_toggle_category {
                             match self.settings_scope {
                                SettingsScope::Profile => {
                                    for alias in session.alias_manager.aliases.values_mut() {
                                        if alias.category == cat { alias.enabled = enabled; }
                                    }
                                },
                                SettingsScope::Global => {
                                    for alias in self.global_config.global_aliases.iter_mut() {
                                        if alias.category == cat { alias.enabled = enabled; }
                                    }
                                }
                            }
                            needs_save = true;
                        }

                        if let Some((name, enabled)) = to_toggle_name {
                             match self.settings_scope {
                                SettingsScope::Profile => {
                                    if let Some(alias) = session.alias_manager.aliases.get_mut(&name) {
                                        alias.enabled = enabled;
                                        needs_save = true;
                                    }
                                },
                                SettingsScope::Global => {
                                    if let Some(alias) = self.global_config.global_aliases.iter_mut().find(|a| a.name == name) {
                                        alias.enabled = enabled;
                                        needs_save = true;
                                    }
                                }
                            }
                        }

                        if let Some(name) = to_delete {
                            match self.settings_scope {
                                SettingsScope::Profile => { session.alias_manager.remove(&name); },
                                SettingsScope::Global => { 
                                    self.global_config.global_aliases.retain(|a| a.name != name); 
                                }
                            }
                            needs_save = true;
                        }

                        if let Some((name, pattern, replacement, category, is_script)) = to_edit {
                            self.editing_alias_name = Some(name.clone());
                            self.alias_edit_name = name;
                            self.alias_edit_pattern = pattern;
                            self.alias_edit_replacement = replacement;
                            self.alias_edit_category = category;
                            self.alias_edit_is_script = is_script;
                            self.show_alias_window = true;
                        }

                        // 處理範圍操作
                        if let Some(op) = op_action {
                            match op {
                                AliasOp::Clone(name) => {
                                    if let Some(a) = session.alias_manager.aliases.get(&name) {
                                        let copy_name = format!("{}_copy", a.name);
                                        let mut new_alias = Alias::new(&copy_name, &a.pattern, &a.replacement);
                                        new_alias.is_script = a.is_script;
                                        new_alias.enabled = a.enabled;
                                        new_alias.category = a.category.clone();
                                        session.alias_manager.add(new_alias);
                                        needs_save = true;
                                    }
                                },
                                AliasOp::MoveToGlobal(name) | AliasOp::CopyToGlobal(name) => {
                                    if let Some(a) = session.alias_manager.aliases.get(&name) {
                                        let new_config = crate::config::AliasConfig {
                                            name: a.name.clone(),
                                            pattern: a.pattern.clone(),
                                            replacement: a.replacement.clone(),
                                            category: a.category.clone(),
                                            is_script: a.is_script,
                                            enabled: a.enabled,
                                        };

                                        if let Some(existing) = self.global_config.global_aliases.iter_mut().find(|ga| ga.name == name) {
                                            *existing = new_config;
                                        } else {
                                            self.global_config.global_aliases.push(new_config);
                                        }
                                        needs_save = true;
                                    }
                                },
                                AliasOp::MoveToProfile(name) => {
                                    self.global_config.global_aliases.retain(|a| a.name != name);
                                    needs_save = true;
                                },
                                AliasOp::RevertToGlobal(name) => {
                                    if let Some(ga) = self.global_config.global_aliases.iter().find(|a| a.name == name) {
                                       let mut alias = mudcore::Alias::new(&ga.name, &ga.pattern, &ga.replacement)
                                           .as_script(ga.is_script);
                                       alias.enabled = ga.enabled;
                                       if let Some(ref cat) = ga.category {
                                           alias = alias.with_category(cat);
                                       }
                                       session.alias_manager.add(alias);
                                       needs_save = true;
                                    }
                                }
                            }
                        }
                    }
                    SettingsTab::Trigger => {
                        ui.horizontal(|ui| {
                            ui.heading(match self.settings_scope {
                                SettingsScope::Profile => "觸發器管理 (Profile)",
                                SettingsScope::Global => "觸發器管理 (Global)",
                            });
                            
                            // 搜尋框
                            ui.add(TextEdit::singleline(&mut self.trigger_search_text).hint_text("🔍 搜尋名稱或內容..."));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("➕ 新增觸發器").clicked() {
                                    self.editing_trigger_name = Some(String::new());
                                    self.trigger_edit_name = String::new();
                                    self.trigger_edit_pattern = String::new();
                                    self.trigger_edit_action = String::new();
                                    self.trigger_edit_category = String::new();
                                    self.trigger_edit_is_script = false;
                                    self.trigger_edit_pattern_type = "auto".to_string();
                                    self.show_trigger_window = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        
                        // 定義觸發器來源類型
                        #[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
                        enum TriggerSource {
                            Global,   // 來自全域設定 (繼承)
                            Profile,  // 本地設定 (獨有)
                            Override, // 本地設定 (覆蓋全域)
                        }

                        // 收集 Trigger 列表 (name, pattern_text, clean_pattern, category, enabled, is_script, action_str, source, pattern_type)
                        let mut trigger_list: Vec<(String, String, String, Option<String>, bool, bool, String, TriggerSource, String)> = match self.settings_scope {
                            SettingsScope::Profile => {
                                session.trigger_manager.order.iter()
                                    .filter_map(|name| {
                                        session.trigger_manager.triggers.get(name).map(|t| {
                                            let (pattern_text, pattern_type_str) = match &t.pattern {
                                                TriggerPattern::Contains(s) => (format!("包含: {}", s), "contains"),
                                                TriggerPattern::StartsWith(s) => (format!("開頭: {}", s), "startswith"),
                                                TriggerPattern::EndsWith(s) => (format!("結尾: {}", s), "endswith"),
                                                TriggerPattern::Regex(s) => (format!("正則: {}", s), "regex"),
                                            };
                                            let clean_pattern = match &t.pattern {
                                                TriggerPattern::Contains(s) | TriggerPattern::StartsWith(s) |
                                                TriggerPattern::EndsWith(s) | TriggerPattern::Regex(s) => s.clone(),
                                            };
                                            let (action_str, is_script) = t.actions.iter().find_map(|a| {
                                                match a {
                                                    TriggerAction::SendCommand(cmd) => Some((cmd.clone(), false)),
                                                    TriggerAction::ExecuteScript(code) => Some((code.clone(), true)),
                                                    _ => None,
                                                }
                                            }).unwrap_or_default();

                                            // 判斷來源
                                            let source = if let Some(global_t) = self.global_config.global_triggers.iter().find(|gt| gt.name == t.name) {
                                                let global_is_match = clean_pattern_string(&global_t.pattern) == clean_pattern &&
                                                                    global_t.action == action_str &&
                                                                    global_t.is_script == is_script &&
                                                                    global_t.enabled == t.enabled &&
                                                                    global_t.category == t.category;

                                                if global_is_match {
                                                    TriggerSource::Global
                                                } else {
                                                    TriggerSource::Override
                                                }
                                            } else {
                                                TriggerSource::Profile
                                            };

                                            (t.name.clone(), pattern_text, clean_pattern, t.category.clone(), t.enabled, is_script, action_str, source, pattern_type_str.to_string())
                                        })
                                    })
                                    .collect()
                            },
                            SettingsScope::Global => {
                                self.global_config.global_triggers.iter().map(|t| {
                                    let pattern_text = format!("(Global) {}", t.pattern);
                                    let pt = t.pattern_type.clone().unwrap_or_else(|| "auto".to_string());
                                    (t.name.clone(), pattern_text, t.pattern.clone(), t.category.clone(), t.enabled, t.is_script, t.action.clone(), TriggerSource::Global, pt)
                                }).collect()
                            }
                        };

                        // 搜尋過濾
                        let search = self.trigger_search_text.to_lowercase();
                        if !search.is_empty() {
                            trigger_list.retain(|(name, p_text, _, cat, _, _, _, _, _)| {
                                name.to_lowercase().contains(&search) ||
                                p_text.to_lowercase().contains(&search) ||
                                cat.as_deref().unwrap_or("").to_lowercase().contains(&search)
                            });
                        }

                        // Grouping Logic
                        let mut grouped_triggers: std::collections::BTreeMap<Option<String>, Vec<(String, String, String, Option<String>, bool, bool, String, TriggerSource, String)>> = std::collections::BTreeMap::new();
                        for item in trigger_list {
                            grouped_triggers.entry(item.3.clone()).or_default().push(item);
                        }

                        let mut to_delete: Option<String> = None;
                        let mut to_edit: Option<(String, String, String, bool, String, String)> = None;
                        let mut to_toggle_name: Option<(String, bool)> = None;
                        let mut to_toggle_category: Option<(Option<String>, bool)> = None;
                        
                        // 操作 Action
                        enum TriggerOp {
                            MoveToGlobal(String),
                            MoveToProfile(String),
                            RevertToGlobal(String),
                            CopyToGlobal(String),
                            Clone(String),
                        }
                        let mut op_action: Option<TriggerOp> = None;
                        let mut set_pending_delete: Option<Option<String>> = None;

                        // 表格繪製
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::auto()) // Enabled / Toggle
                            .column(Column::auto()) // Source Icon
                            .column(Column::initial(150.0).resizable(true)) // Name
                            .column(Column::remainder()) // Pattern
                            .column(Column::auto()) // Actions
                            .header(20.0, |mut header| {
                                header.col(|ui| { ui.strong("啟用"); });
                                header.col(|ui| { ui.strong("來源"); });
                                header.col(|ui| { ui.strong("名稱"); });
                                header.col(|ui| { ui.strong("觸發內容"); });
                                header.col(|ui| { ui.strong("操作"); });
                            })
                            .body(|mut body| {
                                for (category, items) in grouped_triggers {
                                    let category_id_str = category.clone().unwrap_or_else(|| "default".to_string());
                                    let is_expanded_id = body.ui_mut().make_persistent_id(format!("trig_cat_{}", category_id_str));
                                    let is_expanded = body.ui_mut().data(|d| d.get_temp::<bool>(is_expanded_id).unwrap_or(false));

                                    // Group Header Row
                                    body.row(24.0, |mut row| {
                                        row.col(|ui| {
                                            let icon = if is_expanded { "▼" } else { "▶" };
                                            if ui.button(icon).clicked() {
                                                ui.data_mut(|d| d.insert_temp(is_expanded_id, !is_expanded));
                                            }
                                        });
                                        row.col(|_| {}); // Source placeholder
                                        row.col(|ui| {
                                            let cat_name = category.as_deref().unwrap_or("未分類");
                                            ui.strong(cat_name);
                                        });
                                        row.col(|ui| {
                                            // Batch toggle category enabled
                                            if !items.is_empty() {
                                                let all_enabled = items.iter().all(|i| i.4);
                                                let mut current_all = all_enabled;
                                                if ui.checkbox(&mut current_all, "(全選)").changed() {
                                                    to_toggle_category = Some((category.clone(), current_all));
                                                }
                                            }
                                        });
                                        row.col(|_| {}); // Action placeholder
                                    });

                                    if is_expanded {
                                        for (name, pattern_text, clean_pattern, cat, enabled, is_script, action_str, source, pattern_type) in items {
                                            body.row(24.0, |mut row| {
                                                // 停用項目文字變暗
                                                let text_color = if enabled {
                                                    Color32::WHITE
                                                } else {
                                                    Color32::from_gray(100)
                                                };

                                                // 1. 啟用
                                                row.col(|ui| {
                                                    let mut is_enabled = enabled;
                                                    if ui.checkbox(&mut is_enabled, "").changed() {
                                                        to_toggle_name = Some((name.clone(), is_enabled));
                                                    }
                                                });

                                                // 2. 來源圖示
                                                row.col(|ui| {
                                                    match source {
                                                        TriggerSource::Global => { ui.label("🌍").on_hover_text("全域設定 (Global)"); },
                                                        TriggerSource::Profile => { ui.label("👤").on_hover_text("Profile 專屬"); },
                                                        TriggerSource::Override => { ui.label("⚠️").on_hover_text("已覆蓋全域設定 (Override)"); },
                                                    }
                                                });

                                                // 3. 名稱
                                                row.col(|ui| {
                                                    let rt = RichText::new(&name).color(text_color);
                                                    ui.label(rt);
                                                });

                                                // 4. 觸發內容
                                                row.col(|ui| {
                                                    let mut rt = RichText::new(&pattern_text).color(text_color);
                                                    if is_script { rt = rt.italics(); }
                                                    ui.label(rt).on_hover_text(&pattern_text);
                                                });

                                                // 5. 操作
                                                row.col(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 8.0;

                                                        if ui.button("✏️").on_hover_text("編輯").clicked() {
                                                            to_edit = Some((name.clone(), clean_pattern.clone(), action_str.clone(), is_script, cat.clone().unwrap_or_default(), pattern_type.clone()));
                                                        }

                                                        if self.settings_scope == SettingsScope::Profile {
                                                            ui.menu_button(" ⋮ ", |ui| {
                                                                ui.set_min_width(120.0);
                                                                if ui.button("📋 複製").clicked() {
                                                                    op_action = Some(TriggerOp::Clone(name.clone()));
                                                                    ui.close_menu();
                                                                }
                                                                ui.separator();
                                                                match source {
                                                                    TriggerSource::Profile => {
                                                                        if ui.button("🌍 移至全域").clicked() {
                                                                            op_action = Some(TriggerOp::MoveToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                        if ui.button("📋 複製至全域").clicked() {
                                                                            op_action = Some(TriggerOp::CopyToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                    },
                                                                    TriggerSource::Global => {
                                                                        if ui.button("👤 獨立為 Profile").clicked() {
                                                                            op_action = Some(TriggerOp::MoveToProfile(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                        if ui.button("✏️ 覆蓋 (Override)").clicked() {
                                                                            to_edit = Some((name.clone(), clean_pattern.clone(), action_str.clone(), is_script, cat.clone().unwrap_or_default(), pattern_type.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                    },
                                                                    TriggerSource::Override => {
                                                                        if ui.button("🔙 還原至全域").clicked() {
                                                                            op_action = Some(TriggerOp::RevertToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                        if ui.button("🌍 更新至全域").clicked() {
                                                                            op_action = Some(TriggerOp::MoveToGlobal(name.clone()));
                                                                            ui.close_menu();
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                        }

                                                        // 兩段式刪除確認
                                                        if self.pending_trigger_delete.as_deref() == Some(&name) {
                                                            if ui.button(RichText::new("確認").color(Color32::RED)).clicked() {
                                                                to_delete = Some(name.clone());
                                                                set_pending_delete = Some(None);
                                                            }
                                                            if ui.button(RichText::new("取消").color(Color32::GRAY)).clicked() {
                                                                set_pending_delete = Some(None);
                                                            }
                                                        } else if ui.button("🗑️").on_hover_text("刪除").clicked() {
                                                            set_pending_delete = Some(Some(name.clone()));
                                                        }
                                                    });
                                                });
                                            });
                                        }
                                    }
                                }
                            });

                        // Apply pending delete state
                        if let Some(new_val) = set_pending_delete {
                            self.pending_trigger_delete = new_val;
                        }
                        
                        // 處理操作
                        if let Some((cat, enabled)) = to_toggle_category {
                            match self.settings_scope {
                                SettingsScope::Profile => {
                                    for trigger in session.trigger_manager.triggers.values_mut() {
                                        if trigger.category == cat { trigger.enabled = enabled; }
                                    }
                                },
                                SettingsScope::Global => {
                                    for trigger in self.global_config.global_triggers.iter_mut() {
                                        if trigger.category == cat { trigger.enabled = enabled; }
                                    }
                                }
                            }
                            needs_save = true;
                        }

                        if let Some((name, enabled)) = to_toggle_name {
                             match self.settings_scope {
                                SettingsScope::Profile => {
                                    if let Some(trigger) = session.trigger_manager.triggers.get_mut(&name) {
                                        trigger.enabled = enabled;
                                        needs_save = true;
                                    }
                                },
                                SettingsScope::Global => {
                                    if let Some(trigger) = self.global_config.global_triggers.iter_mut().find(|t| t.name == name) {
                                        trigger.enabled = enabled;
                                        needs_save = true;
                                    }
                                }
                            }
                        }

                        if let Some(name) = to_delete {
                            match self.settings_scope {
                                SettingsScope::Profile => { session.trigger_manager.remove(&name); },
                                SettingsScope::Global => { self.global_config.global_triggers.retain(|t| t.name != name); }
                            }
                            needs_save = true;
                        }

                        if let Some((name, pattern, action, is_script, category, pattern_type)) = to_edit {
                            self.editing_trigger_name = Some(name.clone());
                            self.trigger_edit_name = name;
                            self.trigger_edit_pattern = pattern;
                            self.trigger_edit_action = action;
                            self.trigger_edit_category = category;
                            self.trigger_edit_is_script = is_script;
                            self.trigger_edit_pattern_type = pattern_type;
                            self.show_trigger_window = true;
                        }

                        // 處理範圍操作
                        if let Some(op) = op_action {
                            match op {
                                TriggerOp::Clone(name) => {
                                    if let Some(t) = session.trigger_manager.get(&name) {
                                        let copy_name = format!("{}_copy", t.name);
                                        let mut new_trigger = Trigger::new(&copy_name, t.pattern.clone());
                                        for action in &t.actions {
                                            new_trigger.actions.push(action.clone());
                                        }
                                        new_trigger.enabled = t.enabled;
                                        new_trigger.category = t.category.clone();
                                        session.trigger_manager.add(new_trigger);
                                        needs_save = true;
                                    }
                                },
                                TriggerOp::MoveToGlobal(name) | TriggerOp::CopyToGlobal(name) => {
                                    if let Some(t) = session.trigger_manager.get(&name) {
                                        let (action_str, is_script) = t.actions.iter().find_map(|a| {
                                            match a {
                                                TriggerAction::SendCommand(cmd) => Some((cmd.clone(), false)),
                                                TriggerAction::ExecuteScript(code) => Some((code.clone(), true)),
                                                _ => None,
                                            }
                                        }).unwrap_or_default();

                                        let pattern_type = match &t.pattern {
                                            TriggerPattern::Contains(_) => Some("contains".to_string()),
                                            TriggerPattern::StartsWith(_) => Some("startswith".to_string()),
                                            TriggerPattern::EndsWith(_) => Some("endswith".to_string()),
                                            TriggerPattern::Regex(_) => Some("regex".to_string()),
                                        };

                                        let new_config = crate::config::TriggerConfig {
                                            name: t.name.clone(),
                                            pattern: match &t.pattern {
                                                TriggerPattern::Contains(s) | TriggerPattern::StartsWith(s) |
                                                TriggerPattern::EndsWith(s) | TriggerPattern::Regex(s) => s.clone(),
                                            },
                                            action: action_str,
                                            category: t.category.clone(),
                                            is_script,
                                            enabled: t.enabled,
                                            pattern_type,
                                        };

                                        if let Some(existing) = self.global_config.global_triggers.iter_mut().find(|gt| gt.name == name) {
                                            *existing = new_config;
                                        } else {
                                            self.global_config.global_triggers.push(new_config);
                                        }
                                        needs_save = true;
                                    }
                                },
                                TriggerOp::MoveToProfile(name) => {
                                    self.global_config.global_triggers.retain(|t| t.name != name);
                                    needs_save = true;
                                },
                                TriggerOp::RevertToGlobal(name) => {
                                    if let Some(gt) = self.global_config.global_triggers.iter().find(|t| t.name == name) {
                                        if let Some(trigger) = crate::session::Session::create_trigger_from_config(gt) {
                                            session.trigger_manager.add(trigger);
                                            needs_save = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    SettingsTab::Path => {
                        ui.horizontal(|ui| {
                            ui.heading("路徑管理");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("➕ 新增路徑").clicked() {
                                    self.editing_path_name = Some(String::new());
                                    self.path_edit_name = String::new();
                                    self.path_edit_value = String::new();
                                    self.path_edit_category = String::new();
                                    self.show_path_window = true;
                                }
                            });
                        });
                        ui.add_space(5.0);

                        // 收集路徑列表
                        let path_list: Vec<(String, String, Option<String>)> = {
                            session.path_manager.list().iter()
                                .map(|p| (p.name.clone(), p.value.clone(), p.category.clone()))
                                .collect()
                        };

                        let mut grouped_paths: std::collections::BTreeMap<Option<String>, Vec<(String, String, Option<String>)>> = std::collections::BTreeMap::new();
                        for item in path_list {
                            grouped_paths.entry(item.2.clone()).or_default().push(item);
                        }

                        let mut to_delete: Option<String> = None;
                        let mut to_edit: Option<(String, String, String)> = None;

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if grouped_paths.is_empty() {
                                ui.label("尚無路徑");
                            } else {
                                for (category, items) in grouped_paths {
                                    let category_name = category.as_deref().unwrap_or("未分類");
                                    
                                    egui::CollapsingHeader::new(RichText::new(category_name).strong())
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            for (name, value, cat) in items {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(10.0);
                                                    
                                                    ui.label(format!("{} → {}", name, value));
                                                    
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.small_button("🗑️").clicked() {
                                                            to_delete = Some(name.clone());
                                                        }
                                                        if ui.small_button("✏️").clicked() {
                                                            to_edit = Some((name.clone(), value.clone(), cat.unwrap_or_default()));
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                }
                            }
                        });

                        if let Some(name) = to_delete {
                            session.path_manager.remove(&name);
                            needs_save = true;
                        }
                        if let Some((name, value, category)) = to_edit {
                            self.editing_path_name = Some(name.clone());
                            self.path_edit_name = name;
                            self.path_edit_value = value;
                            self.path_edit_category = category;
                            self.show_path_window = true;
                        }
                    }
                    SettingsTab::Logger => {
                        ui.heading("日誌控制");
                        ui.add_space(10.0);
                        
                        if session.logger.is_recording() {
                            let log_path_str = session.logger.path().map(|p| p.display().to_string()).unwrap_or_default();
                            ui.label(format!("狀態: 正在記錄中 ({})", &log_path_str));
                            ui.horizontal(|ui| {
                                if ui.button("停止記錄").clicked() {
                                    let _ = session.logger.stop();
                                }
                                if ui.button("📂 開啟日誌檔").clicked() {
                                    if let Some(p) = session.logger.path().map(|p| p.to_path_buf()) {
                                        let _ = session.logger.flush();
                                        let _ = std::process::Command::new("open").arg(&p).spawn();
                                    }
                                }
                            });
                        } else {
                            ui.label("狀態: 未啟動");
                            if ui.button("開始記錄").clicked() {
                                let path = format!("logs/mud_log_{}.txt", chrono_lite_timestamp());
                                let _ = session.logger.start(&path);
                            }
                        }
                    }
                    SettingsTab::General => {
                        ui.heading("一般設定");
                        ui.add_space(10.0);

                        // === 字型大小設定 ===
                        ui.horizontal(|ui| {
                            ui.label("字型大小:");
                            let slider = egui::Slider::new(&mut self.global_config.ui.font_size, 10.0..=24.0)
                                .step_by(1.0)
                                .suffix(" px");
                            if ui.add(slider).changed() {
                                needs_save = true;
                            }
                            if ui.button("重置").clicked() {
                                self.global_config.ui.font_size = 14.0;
                                needs_save = true;
                            }
                        });
                        ui.label(
                            RichText::new("💡 也可使用 ⌘+/⌘- 快速調整，⌘0 重置")
                                .small()
                                .color(Color32::GRAY)
                        );
                        ui.add_space(5.0);

                        ui.checkbox(&mut session.auto_scroll, "自動捲動畫面");
                        ui.add_space(5.0);

                        // === 自動重連延遲 ===
                        ui.horizontal(|ui| {
                            ui.label("重連延遲:");
                            let slider = egui::Slider::new(&mut self.global_config.ui.reconnect_delay_secs, 1..=60)
                                .suffix(" 秒");
                            if ui.add(slider).changed() {
                                needs_save = true;
                            }
                        });
                        ui.add_space(5.0);

                        ui.label(format!("畫面單字字典: {} 個單字", session.screen_words.len()));
                        ui.label(format!("指令字典: {} 個指令", session.command_dict.len()));

                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(5.0);

                        // === 九宮格快捷鍵設定 ===
                        ui.heading("九宮格快捷鍵");
                        ui.add_space(5.0);

                        let mut numpad_changed = false;
                        if ui.checkbox(&mut self.global_config.numpad.enabled, "啟用九宮格行走模式").changed() {
                            numpad_changed = true;
                        }
                        ui.label(
                            RichText::new("⚠ 開啟後，數字鍵將直接發送指令而非輸入文字")
                                .small()
                                .color(Color32::YELLOW)
                        );
                        ui.add_space(8.0);

                        // 九宮格排列的指令編輯
                        egui::Grid::new("numpad_grid")
                            .num_columns(3)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                // Row 1: 7 8 9
                                ui.horizontal(|ui| {
                                    ui.label("7:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_7).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("8:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_8).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("9:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_9).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.end_row();

                                // Row 2: 4 5 6
                                ui.horizontal(|ui| {
                                    ui.label("4:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_4).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("5:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_5).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("6:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_6).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.end_row();

                                // Row 3: 1 2 3
                                ui.horizontal(|ui| {
                                    ui.label("1:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_1).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("2:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_2).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("3:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_3).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.end_row();

                                // Row 4: 0 . (empty)
                                ui.horizontal(|ui| {
                                    ui.label("0:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_0).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(".:");
                                    if ui.add(TextEdit::singleline(&mut self.global_config.numpad.key_dot).desired_width(80.0)).changed() { numpad_changed = true; }
                                });
                                ui.label(""); // placeholder
                                ui.end_row();
                            });

                        if numpad_changed {
                            needs_save = true;
                        }
                    }
                }
                
                ui.add_space(10.0);
                ui.separator();
                if ui.button("關閉").clicked() {
                    should_close = true;
                }
            });
        
        if needs_save {
            self.save_config();
        }
        if should_close {
            self.show_settings_window = false;
        }
    }
}
