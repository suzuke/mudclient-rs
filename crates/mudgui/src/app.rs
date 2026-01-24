//! MUD Client 主要 UI 邏輯

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, TextEdit};
use eframe::egui::text::LayoutJob;
use mudcore::{
    Alias, TelnetClient, Trigger, TriggerAction,
    TriggerPattern,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::ansi::parse_ansi;
use crate::config::{GlobalConfig, ProfileManager, TriggerConfig};
use crate::session::SessionManager;


/// MUD 客戶端 GUI 應用程式
pub struct MudApp {
    /// Tokio 運行時
    runtime: Runtime,

    /// 當前設定頁面標籤
    settings_tab: SettingsTab,

    // === 多帳號系統 ===
    /// Profile 管理器
    profile_manager: ProfileManager,
    /// Session 管理器
    session_manager: SessionManager,
    /// 全域設定
    global_config: GlobalConfig,
    /// 是否顯示 Profile 選擇視窗
    show_profile_window: bool,
    /// 待連線的 Profile 名稱（用於在 UI 循環外處理連線）
    pending_connect_profile: Option<String>,

    // === UI 臨時狀態 ===
    /// 當前選中的視窗 ID
    active_window_id: String,
    
    // === 別名編輯狀態 ===
    show_alias_window: bool,
    editing_alias_name: Option<String>,
    alias_edit_pattern: String,
    alias_edit_replacement: String,
    alias_edit_category: String,

    // === 觸發器編輯狀態 ===
    show_trigger_window: bool,
    editing_trigger_name: Option<String>,
    trigger_edit_name: String,
    trigger_edit_pattern: String,
    trigger_edit_action: String,
    trigger_edit_category: String,
    trigger_edit_is_script: bool,

    /// 設定視窗開關
    show_settings_window: bool,
}

/// 設定中心標籤頁
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Alias,
    Trigger,
    Logger,
    General,
}

/// 發送給網路執行緒的命令
#[derive(Debug)]
#[allow(dead_code)]
enum Command {
    Connect(String, u16),
    Send(String),
    Disconnect,
}

#[allow(dead_code)]
impl MudApp {
    /// 創建新的 MUD 客戶端應用程式
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 設定字型
        Self::configure_fonts(&cc.egui_ctx);

        // 創建 Tokio 運行時
        let runtime = Runtime::new().expect("無法創建 Tokio 運行時");

        Self {
            runtime,
            settings_tab: SettingsTab::Alias,
            // 多帳號系統
            profile_manager: ProfileManager::new(),
            session_manager: SessionManager::new(),
            global_config: GlobalConfig::load(),
            show_profile_window: false,
            pending_connect_profile: None,

            // UI 狀態
            active_window_id: "main".to_string(),
            show_alias_window: false,
            editing_alias_name: None,
            alias_edit_pattern: String::new(),
            alias_edit_replacement: String::new(),
            alias_edit_category: String::new(),
            show_trigger_window: false,
            editing_trigger_name: None,
            trigger_edit_name: String::new(),
            trigger_edit_pattern: String::new(),
            trigger_edit_action: String::new(),
            trigger_edit_category: String::new(),
            trigger_edit_is_script: false,
            show_settings_window: false,
        }
    }

    /// 儲存設定到檔案
    fn save_config(&mut self) {
        // 如果有活躍 Session，將其目前狀態同步回 Profile
        if let Some(session) = self.session_manager.active_session() {
            let profile_name = session.profile_name.clone();
            
            // 1. 同步 Alias
            let mut new_aliases = Vec::new();
            for name in &session.alias_manager.sorted_aliases {
                if let Some(a) = session.alias_manager.get(name) {
                    new_aliases.push(crate::config::AliasConfig {
                        name: a.name.clone(),
                        pattern: a.pattern.clone(),
                        replacement: a.replacement.clone(),
                        category: a.category.clone(),
                        enabled: a.enabled,
                    });
                }
            }

            // 2. 同步 Trigger
            let mut new_triggers = Vec::new();
            for name in &session.trigger_manager.order {
                 if let Some(t) = session.trigger_manager.get(name) {
                     let (action_str, is_script) = if let Some(first_action) = t.actions.first() {
                         match first_action {
                             TriggerAction::SendCommand(s) => (s.clone(), false),
                             TriggerAction::ExecuteScript(s) => (s.clone(), true),
                             _ => (String::new(), false),
                         }
                     } else {
                         (String::new(), false)
                     };
                     
                     let pat_str = match &t.pattern {
                         TriggerPattern::Contains(s) | TriggerPattern::StartsWith(s) | TriggerPattern::EndsWith(s) | TriggerPattern::Regex(s) => s.clone(),
                     };
                     
                     new_triggers.push(crate::config::TriggerConfig {
                         name: t.name.clone(),
                         pattern: pat_str,
                         action: action_str,
                         category: t.category.clone(),
                         is_script,
                         enabled: t.enabled,
                     });
                 }
            }

            // 3. 更新 ProfileManager 並儲存
             if let Some(profile) = self.profile_manager.get_mut(&profile_name) {
                 profile.aliases = new_aliases;
                 profile.triggers = new_triggers;
                 
                 // 儲存到磁碟
                 let p = profile.clone();
                 if let Err(e) = self.profile_manager.save(p) {
                     tracing::error!("Failed to save profile {}: {}", profile_name, e);
                 } else {
                     tracing::info!("Saved profile: {}", profile_name);
                 }
             }
        }
        
        // 儲存全域設定
        if let Err(e) = self.global_config.save() {
            tracing::error!("Failed to save global config: {}", e);
        }
    }

    /// 設定字型（支援中文）
    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // 嘗試載入系統中文字型作為 fallback
        if let Some(cjk_font_data) = Self::load_system_cjk_font() {
            fonts.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(cjk_font_data)),
            );

            // 設定字型優先順序
            // 強制將 CJK 字型放在最前面，確保嚴格對齊 (犧牲部分英數美觀)
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "cjk".to_owned());
            } else {
                fonts.families.insert(
                    egui::FontFamily::Monospace,
                    vec![
                        "cjk".to_owned(),
                        "Monaco".to_owned(),
                        "Hack".to_owned(),
                        "Ubuntu-Mono".to_owned(),
                    ],
                );
            }

            // Proportional: 作為 fallback 添加到最後
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("cjk".to_owned());

            tracing::info!("已載入系統中文字型");
        } else {
            tracing::warn!("無法載入系統中文字型，中文可能無法正確顯示");
        }

        ctx.set_fonts(fonts);
    }

    /// 載入系統 CJK 字型
    fn load_system_cjk_font() -> Option<Vec<u8>> {
        use font_kit::family_name::FamilyName;
        use font_kit::properties::Properties;
        use font_kit::source::SystemSource;

        let source = SystemSource::new();

        // 嘗試載入常見的中文字型（優先使用等寬字型以解決對齊問題）
        let font_names = [
            // 優先：現代等寬中文字型 (如果使用者有安裝)
            FamilyName::Title("Noto Sans Mono CJK TC".to_string()),
            FamilyName::Title("Noto Sans Mono CJK SC".to_string()),
            FamilyName::Title("Sarasa Mono TC".to_string()),
            // 優先備選：macOS 嚴格等寬字型 (雖然較舊但對齊準確)
            FamilyName::Title("LiHei Pro".to_string()),           // 儷黑 Pro (舊名)
            FamilyName::Title("Apple LiGothic Medium".to_string()), // 儷黑 Pro (新名)
            FamilyName::Title("MingLiU".to_string()),             // 細明體 (Windows 移植)
            FamilyName::Title("PMingLiU".to_string()),            // 新細明體
            FamilyName::Title("BiauKai".to_string()),             // 標楷體
            FamilyName::Title("Lisong Pro".to_string()),          // 儷宋 Pro
            // 再次備選：冬青黑體/華文黑體
            FamilyName::Title("Hiragino Sans GB".to_string()), 
            FamilyName::Title("STHeiti TC".to_string()),       
            FamilyName::Title("STHeiti SC".to_string()),   
            FamilyName::Title("Heiti TC".to_string()),         
            FamilyName::Title("Heiti SC".to_string()),
            // 最後 fallback
            // 系統預設黑體 (macOS 標準) - 雖然不是嚴格等寬，但比舊式字型美觀
            FamilyName::Title("PingFang TC".to_string()),
            FamilyName::Title("PingFang SC".to_string()),
            FamilyName::Title("Microsoft JhengHei".to_string()),
            FamilyName::Title("WenQuanYi Micro Hei".to_string()),
        ];

        for family in font_names {
            if let Ok(handle) = source.select_best_match(&[family], &Properties::new()) {
                if let Ok(font) = handle.load() {
                    if let Some(data) = font.copy_font_data() {
                        tracing::info!("找到字型: {:?}", font.full_name());
                        return Some((*data).clone());
                    }
                }
            }
        }

        None
    }

    /// 從 Profile 建立連線
    fn connect_to_profile(&mut self, profile_name: &str, ctx: egui::Context) {
        // 從 ProfileManager 取得 Profile
        if let Some(profile) = self.profile_manager.get(profile_name) {
            tracing::info!("建立 Profile 連線: {}", profile_name);
            
            // 建立新的 Session
            let session_id = self.session_manager.create_session(profile);
            
            // 啟動連線
            self.start_connection(session_id, ctx);
            
            // 顯示本地訊息
            if let Some(session) = self.session_manager.get_mut(session_id) {
                session.handle_text(&format!(">>> 已建立 Profile 會話: {} ({}:{})\n", profile_name, session.host, session.port), true);
            }
        } else {
            tracing::warn!("找不到 Profile: {}", profile_name);
        }
    }

    /// 從 Profile 設定建立 Trigger
    fn create_trigger_from_profile_config(config: &TriggerConfig) -> Option<Trigger> {        
        // 建立 Pattern
        let pattern = TriggerPattern::Regex(config.pattern.clone());
        
        // 建立 Trigger
        let mut trigger = Trigger::new(config.name.clone(), pattern);
        trigger.enabled = config.enabled;
        
        // 根據 is_script 判斷 action 類型
        let action = if config.is_script {
            TriggerAction::ExecuteScript(config.action.clone())
        } else {
            TriggerAction::SendCommand(config.action.clone())
        };
        trigger.actions.push(action);
        
        Some(trigger)
    }

    /// 啟動指定 Session 的網路連線
    fn start_connection(&mut self, session_id: crate::session::SessionId, ctx: egui::Context) {
        let (host, port) = {
            let session = match self.session_manager.get(session_id) {
                Some(s) => s,
                None => return,
            };
            (session.host.clone(), session.port.parse::<u16>().unwrap_or(7777))
        };

        // 創建 channels
        use crate::session::Command as SessionCommand;
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<SessionCommand>(32);
        let (msg_tx, msg_rx) = mpsc::channel::<String>(1024);

        if let Some(session) = self.session_manager.get_mut(session_id) {
            session.command_tx = Some(cmd_tx.clone());
            session.message_rx = Some(msg_rx);
            session.status = crate::session::ConnectionStatus::Connecting;
        }

        // 啟動網路執行緒
        self.runtime.spawn(async move {
            let mut client = TelnetClient::default();

            // 處理命令
            loop {
                tokio::select! {
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            SessionCommand::Connect(h, p) => {
                                match client.connect(&h, p).await {
                                    Ok(_) => {
                                        let _ = msg_tx.send(format!(">>> 已連線到 {}:{}\n", h, p)).await;

                                        // 開始讀取迴圈
                                        loop {
                                            tokio::select! {
                                                result = client.read() => {
                                                    match result {
                                                        Ok(text) if !text.is_empty() => {
                                                            let _ = msg_tx.send(text).await;
                                                            ctx.request_repaint();
                                                        }
                                                        Ok(_) => {
                                                            let _ = msg_tx.send(">>> 連線已關閉\n".to_string()).await;
                                                            break;
                                                        }
                                                        Err(e) => {
                                                            let _ = msg_tx.send(format!(">>> 錯誤: {}\n", e)).await;
                                                            break;
                                                        }
                                                    }
                                                }
                                                Some(cmd) = cmd_rx.recv() => {
                                                    match cmd {
                                                        SessionCommand::Send(text) => {
                                                            if let Err(e) = client.send(&text).await {
                                                                let _ = msg_tx.send(format!(">>> 發送失敗: {}\n", e)).await;
                                                            }
                                                        }
                                                        SessionCommand::Disconnect => {
                                                            client.disconnect().await;
                                                            let _ = msg_tx.send(">>> 已斷開連線\n".to_string()).await;
                                                            break;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = msg_tx.send(format!(">>> 連線失敗: {}\n", e)).await;
                                    }
                                }
                            }
                            SessionCommand::Disconnect => break,
                            _ => {}
                        }
                    }
                    else => break,
                }
            }
        });

        // 發送初始連線命令
        let _ = cmd_tx.blocking_send(SessionCommand::Connect(host, port));
    }

    /// 發送訊息（針對指定 Session）
    fn send_message_for_session(&mut self, session: &mut crate::session::Session) {
        // 發送指令時自動捲到最底
        session.scroll_to_bottom_on_next_frame = true;
        
        let text = session.input.clone();

        // 只有非空訊息才儲存到歷史
        if !text.is_empty() {
            session.input_history.push(text.clone());
        }
        session.history_index = None;

        // 別名處理
        let clean_text = crate::ansi::strip_ansi(&text);
        let expanded = session.alias_manager.process(&clean_text);

        // 處理本地回顯與觸發
        if expanded.is_empty() {
            session.handle_text("\n", true);
        } else {
            session.handle_text(&format!("{}\n", text), true);
        }

        // 最後處理發送 (這需要持有租用 session.command_tx)
        if let Some(tx) = &session.command_tx {
            if expanded.is_empty() {
                let _ = tx.blocking_send(crate::session::Command::Send(String::new()));
            } else {
                // 如果別名展開後包含多個命令（以分號分隔），則分開發送
                for cmd in expanded.split(';') {
                    let cmd = cmd.trim();
                    if !cmd.is_empty() {
                        let _ = tx.blocking_send(crate::session::Command::Send(cmd.to_string()));
                    }
                }
            }
        }
    }

    /// 檢查所有 Session 並執行自動重連
    fn check_reconnect(&mut self, ctx: &egui::Context) {
        let mut to_reconnect = Vec::new();
        
        for session in self.session_manager.sessions() {
            if let crate::session::ConnectionStatus::Reconnecting = session.status {
                if let Some(until) = session.reconnect_delay_until {
                    if Instant::now() >= until {
                        to_reconnect.push(session.id);
                    } else {
                        // 持續刷新 UI 以更新倒數顯示
                        ctx.request_repaint();
                    }
                }
            }
        }
        
        for id in to_reconnect {
            self.start_connection(id, ctx.clone());
        }
    }

    /// 處理所有活躍 Session 的網路訊息
    fn process_messages(&mut self) {
        let session_ids: Vec<_> = self.session_manager.sessions().iter().map(|s| s.id).collect();

        for id in session_ids {
            // 首先收集訊息，避免借用衝突
            let messages = if let Some(session) = self.session_manager.get_mut(id) {
                if let Some(ref mut rx) = session.message_rx {
                    let mut msgs = Vec::new();
                    while let Ok(msg) = rx.try_recv() {
                        msgs.push(msg);
                    }
                    msgs
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            // 處理收集到的訊息
            if !messages.is_empty() {
                if let Some(session) = self.session_manager.get_mut(id) {
                    for msg in messages {
                        session.handle_text(&msg, false);

                        use crate::session::ConnectionStatus as SessionStatus;
                        if msg.contains("已連線到") {
                            let info = msg.replace(">>> 已連線到 ", "").replace("\n", "");
                            session.status = SessionStatus::Connected(info);
                            session.connected_at = Some(Instant::now());
                        } else if msg.contains("連線已關閉") || msg.contains("已斷開連線") {
                            session.connected_at = None;
                            if session.auto_reconnect {
                                use std::time::Duration;
                                session.reconnect_delay_until = Some(Instant::now() + Duration::from_secs(3));
                                session.status = SessionStatus::Reconnecting;
                            } else {
                                session.status = SessionStatus::Disconnected;
                            }
                        }
                    }
                }
            }
        }
    }

    /// 繪製訊息顯示區（支援 ANSI 顏色）
    fn render_message_area(ui: &mut egui::Ui, session: &mut crate::session::Session, active_window_id: &str) {
        let available_height = ui.available_height() - 40.0; // 保留輸入區空間

        // 檢查是否需要強制捲到底部
        let force_scroll_to_bottom = session.scroll_to_bottom_on_next_frame;
        session.scroll_to_bottom_on_next_frame = false;

        // 使用固定 ID 以便後續操作 State
        let scroll_area_id = egui::Id::new("main_message_scroll_area");

        let output = ScrollArea::vertical()
            .id_salt(scroll_area_id)
            .auto_shrink([false, false])
            .max_height(available_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let font_id = FontId::monospace(14.0);

                if let Some(window) = session.window_manager.get(active_window_id) {
                    for msg in window.messages() {
                        // 解析 ANSI 顏色碼
                        let spans = parse_ansi(&msg.content);
                        
                        // 使用 LayoutJob 來正確渲染多顏色文字
                        let mut job = LayoutJob::default();
                        
                        for span in spans {
                            let color = span.fg_color;
                            let background = span.bg_color.unwrap_or(Color32::TRANSPARENT);
                            let italics = span.blink; // 使用斜體來標示閃爍
                            
                            job.append(
                                &span.text,
                                0.0,
                                egui::TextFormat {
                                    font_id: font_id.clone(),
                                    color,
                                    background,
                                    italics,
                                    ..Default::default()
                                },
                            );
                        }
                        
                        ui.label(job);
                    }
                }
            });

        // 如果需要強制捲到底部，直接設定 offset
        if force_scroll_to_bottom {
            let content_size = output.content_size;
            let inner_rect = output.inner_rect;
            let max_scroll = (content_size.y - inner_rect.height()).max(0.0);
            
            // 載入並修改 state
            if let Some(mut state) = egui::scroll_area::State::load(ui.ctx(), output.id) {
                state.offset.y = max_scroll;
                state.store(ui.ctx(), output.id);
            }
        }
    }


    /// 繪製別名編輯介面
    fn render_alias_edit(
        ctx: &egui::Context,
        session_opt: Option<&mut crate::session::Session>,
        editing_alias_name: &mut Option<String>,
        alias_edit_pattern: &mut String,
        alias_edit_replacement: &mut String,
        alias_edit_category: &mut String,
        show_alias_window: &mut bool,
        needs_save_flag: &mut bool,
    ) {
        egui::Window::new(if editing_alias_name.as_ref().map_or(true, |n| n.is_empty()) { "➕ 新增別名" } else { "✏️ 編輯別名" })
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("觸發詞:");
                    ui.text_edit_singleline(alias_edit_pattern);
                });

                ui.horizontal(|ui| {
                    ui.label("替換為:");
                    ui.text_edit_singleline(alias_edit_replacement);
                });

                ui.horizontal(|ui| {
                    ui.label("分類:");
                    ui.text_edit_singleline(alias_edit_category);
                });

                ui.add_space(10.0);
                ui.label("提示: 使用 $1, $2 等作為參數佔位符");

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("💾 儲存").clicked() {
                        if !alias_edit_pattern.is_empty() {
                            if let Some(session) = session_opt {
                                // 如果是編輯模式，先刪除舊的
                                if let Some(ref old_name) = editing_alias_name {
                                    if !old_name.is_empty() {
                                        session.alias_manager.remove(old_name);
                                    }
                                }
                                // 新增別名
                                let mut alias = Alias::new(
                                    alias_edit_pattern.clone(),
                                    alias_edit_pattern.clone(),
                                    alias_edit_replacement.clone(),
                                );
                                if !alias_edit_category.is_empty() {
                                    alias.category = Some(alias_edit_category.clone());
                                }
                                session.alias_manager.add(alias);
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
    fn render_trigger_edit(
        ctx: &egui::Context,
        session_opt: Option<&mut crate::session::Session>,
        editing_trigger_name: &mut Option<String>,
        trigger_edit_name: &mut String,
        trigger_edit_pattern: &mut String,
        trigger_edit_action: &mut String,
        trigger_edit_category: &mut String,
        trigger_edit_is_script: &mut bool,
        show_trigger_window: &mut bool,
        needs_save_flag: &mut bool,
    ) {
        egui::Window::new(if editing_trigger_name.as_ref().map_or(true, |n| n.is_empty()) { "➕ 新增觸發器" } else { "✏️ 編輯觸發器" })
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("名稱:");
                    ui.text_edit_singleline(trigger_edit_name);
                });

                ui.horizontal(|ui| {
                    ui.label("匹配文字:");
                    ui.text_edit_singleline(trigger_edit_pattern);
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
                        ui.text_edit_multiline(trigger_edit_action);
                    } else {
                        ui.text_edit_singleline(trigger_edit_action);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("分類標籤:");
                    ui.text_edit_singleline(trigger_edit_category);
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
                            if let Some(session) = session_opt {
                                // 如果是編輯模式，先刪除舊的
                                if let Some(ref old_name) = editing_trigger_name {
                                    if !old_name.is_empty() {
                                        session.trigger_manager.remove(old_name);
                                    }
                                }
                                // 新增觸發器
                                let pattern = if trigger_edit_pattern.contains("(.+)")
                                    || trigger_edit_pattern.contains("(.*)")
                                    || trigger_edit_pattern.contains("\\d")
                                    || trigger_edit_pattern.contains("[")
                                    || trigger_edit_pattern.contains("$")
                                    || trigger_edit_pattern.contains("^")
                                {
                                    TriggerPattern::Regex(trigger_edit_pattern.clone())
                                } else {
                                    TriggerPattern::Contains(trigger_edit_pattern.clone())
                                };
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

    /// 繪製輸入區
    fn render_input_area(ui: &mut egui::Ui, session: &mut crate::session::Session, any_popup_open: bool) {
        ui.horizontal(|ui| {
            let response = ui.add(
                TextEdit::singleline(&mut session.input)
                    .desired_width(ui.available_width())
                    .font(FontId::monospace(14.0))
                    .hint_text("輸入指令..."),
            );

            if !any_popup_open && !response.has_focus() {
                response.request_focus();
            }

            // 按 Enter 發送
            // 按 Enter 發送
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && response.has_focus() {
                // 發送訊息 (即使是空字串也發送，以便在 MUD 中執行重複動作或保持連線)
                let raw_input = session.input.clone();
                let cmds: Vec<&str> = raw_input.split(';').map(|s| s.trim()).collect();
                
                // 如果是空字串，也當作一個空指令發送
                let cmds = if cmds.is_empty() { vec![""] } else { cmds };

                // 記錄歷史 (原始輸入)
                if !raw_input.is_empty() {
                    session.input_history.push(raw_input.clone());
                    if session.input_history.len() > 1000 {
                        session.input_history.remove(0);
                    }
                }
                session.history_index = None;
                
                for cmd in cmds {
                    session.handle_user_input(&cmd.to_string());
                }
                
                // 不清除輸入，而是全選 (方便重複發送)
                // session.input.clear(); 
                
                response.request_focus();
                
                // 強制全選
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(session.input.chars().count()),
                    )));
                    egui::TextEdit::store_state(ui.ctx(), response.id, state);
                }

                // 強制捲動到底部
                session.scroll_to_bottom_on_next_frame = true;
            }

            // 處理 Tab 補齊後的游標移動
            if session.tab_completed {
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                    let char_count = session.input.chars().count();
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(char_count)
                    )));
                    egui::TextEdit::store_state(ui.ctx(), response.id, state);
                }
                session.tab_completed = false;
            }
            
            // 歷史導航（上/下箭頭）與 Tab 補齊
            if response.has_focus() || response.lost_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    Self::navigate_history_for_session(session, -1);
                    session.tab_completion_prefix = None;
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    Self::navigate_history_for_session(session, 1);
                    session.tab_completion_prefix = None;
                }
                // Tab 補齊
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                    Self::tab_complete_for_session(session);
                    ui.ctx().request_repaint();
                }
            }
        });
    }

    /// 導航輸入歷史
    fn navigate_history_for_session(session: &mut crate::session::Session, direction: i32) {
        if session.input_history.is_empty() {
            return;
        }

        let new_index = match session.history_index {
            Some(idx) => {
                let new = idx as i32 + direction;
                if new < 0 {
                    0
                } else if new >= session.input_history.len() as i32 {
                    session.history_index = None;
                    session.input.clear();
                    return;
                } else {
                    new as usize
                }
            }
            None if direction < 0 => session.input_history.len() - 1,
            None => return,
        };

        session.history_index = Some(new_index);
        session.input = session.input_history[new_index].clone();
    }

    /// Tab 補齊邏輯
    fn tab_complete_for_session(session: &mut crate::session::Session) {
        if session.input.is_empty() {
            session.tab_completion_prefix = None;
            session.last_completed_input = None;
            return;
        }

        // 檢查是否發生了手動修改：
        // 如果當前輸入與上次自動補齊後的結果不同，則視為使用者手動修改了內容
        if let Some(ref last_completed) = session.last_completed_input {
            if &session.input != last_completed {
                session.tab_completion_prefix = None;
                session.tab_completion_index = 0;
            }
        }
        
        if let Some(ref prefix) = session.tab_completion_prefix {
            if !session.input.starts_with(prefix) || &session.input == prefix {
                session.tab_completion_prefix = Some(session.input.clone());
                session.tab_completion_index = 0;
                session.tab_completed = false;
            }
        } else {
            session.tab_completion_prefix = Some(session.input.clone());
            session.tab_completion_index = 0;
            session.tab_completed = false;
        }
        
        let original_prefix = session.tab_completion_prefix.clone().unwrap();
        
        let (prefix_to_match, base_input) = if let Some(last_space_idx) = original_prefix.rfind(' ') {
            let (base, last) = original_prefix.split_at(last_space_idx + 1);
            (last.to_string(), Some(base.to_string()))
        } else {
            (original_prefix.clone(), None)
        };

        let mut matches: Vec<String> = Vec::new();
        
        // 1. 補齊歷史指令
        for history in &session.input_history {
            if history.starts_with(&original_prefix) && !matches.contains(history) {
                matches.push(history.clone());
            }
        }
        
        // 2. 補齊畫面單字
        let clean_prefix = prefix_to_match.to_lowercase();
        let mut word_matches: Vec<_> = session.screen_words.iter()
            .filter(|(w, _)| w.to_lowercase().starts_with(&clean_prefix))
            .collect();
            
        word_matches.sort_by(|(a_word, a_meta), (b_word, b_meta)| {
            b_meta.is_mob.cmp(&a_meta.is_mob)
                .then_with(|| b_meta.last_seen.cmp(&a_meta.last_seen))
                .then_with(|| a_word.len().cmp(&b_word.len()))
        });
        
        for (word, _) in word_matches {
            let full_match = if let Some(ref base) = base_input {
                format!("{}{}", base, word)
            } else {
                word.clone()
            };
            if !matches.contains(&full_match) {
                matches.push(full_match);
            }
        }

        if !matches.is_empty() {
            let index = session.tab_completion_index % matches.len();
            session.input = matches[index].clone();
            session.last_completed_input = Some(session.input.clone());
            session.tab_completion_index += 1;
            session.tab_completed = true;
        } else {
            session.last_completed_input = None;
        }
    }

    /// 發送方向指令
    fn send_direction_for_session(session: &mut crate::session::Session, dir: &str) {
        if let Some(tx) = &session.command_tx {
            let _ = tx.blocking_send(crate::session::Command::Send(dir.to_string()));
        }
    }

    /// 處理快捷鍵
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context, pending_action: &mut Option<PendingAction>) {
        ctx.input(|i| {
            // F1-F5 功能鍵
            if i.key_pressed(egui::Key::F2) || i.key_pressed(egui::Key::F3) || i.key_pressed(egui::Key::F4) {
                self.show_settings_window = true;
            }

            // Ctrl+L 清除畫面
            if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                *pending_action = Some(PendingAction::ClearActiveWindow);
            }

            // Escape 關閉所有彈出視窗
            if i.key_pressed(egui::Key::Escape) {
                self.show_settings_window = false;
                self.show_alias_window = false;
                self.show_trigger_window = false;
                self.show_profile_window = false;
            }

            // === 分頁切換快捷鍵 ===
            #[cfg(target_os = "macos")]
            let cmd = i.modifiers.mac_cmd;
            #[cfg(not(target_os = "macos"))]
            let cmd = i.modifiers.ctrl;

            if cmd && !i.modifiers.shift {
                // Cmd+1~9 切換分頁
                let num_keys = [
                    egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
                    egui::Key::Num4, egui::Key::Num5, egui::Key::Num6,
                    egui::Key::Num7, egui::Key::Num8, egui::Key::Num9,
                ];
                for (idx, key) in num_keys.iter().enumerate() {
                    if i.key_pressed(*key) {
                        *pending_action = Some(PendingAction::SwitchTab(idx));
                    }
                }

                // Cmd+[ 上一個分頁
                if i.key_pressed(egui::Key::OpenBracket) {
                    *pending_action = Some(PendingAction::PrevTab);
                }
                // Cmd+] 下一個分頁
                if i.key_pressed(egui::Key::CloseBracket) {
                    *pending_action = Some(PendingAction::NextTab);
                }

                // Cmd+T 開啟連線管理
                if i.key_pressed(egui::Key::T) {
                    self.show_profile_window = true;
                }
            }
        });
    }

    /// 繪製 Profile 管理視窗
    fn render_profile_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("連線管理")
            .resizable(true)
            .default_width(450.0)
            .default_height(350.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Profile 列表");
                ui.separator();

                let profiles: Vec<_> = self.profile_manager.list().iter().map(|p| {
                    (p.name.clone(), p.display_name.clone(), p.connection.host.clone(), p.connection.port.clone())
                }).collect();

                if profiles.is_empty() {
                    ui.label("尚無任何 Profile。");
                    ui.add_space(10.0);
                } else {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (name, display_name, host, port) in &profiles {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(display_name).strong());
                                    ui.label(format!("({}:{})", host, port));
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        // 點擊連線按鈕時設定待連線的 Profile
                                        if ui.button("🔌 連線").clicked() {
                                            self.pending_connect_profile = Some(name.clone());
                                            self.show_profile_window = false;
                                        }
                                    });
                                });
                            });
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
    }

    /// 繪製設定視窗 (獨立 Window)
    fn render_settings_window(&mut self, ctx: &egui::Context) {
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
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Logger, "日誌 (Logger)");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::General, "一般 (General)");
                });
                ui.separator();
                
                // 根據目前的 Tab 渲染內容
                match self.settings_tab {
                    SettingsTab::Alias => {
                        ui.horizontal(|ui| {
                            ui.heading("別名管理");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("➕ 新增別名").clicked() {
                                    self.editing_alias_name = Some(String::new());
                                    self.alias_edit_pattern = String::new();
                                    self.alias_edit_replacement = String::new();
                                    self.alias_edit_category = String::new();
                                    self.show_alias_window = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        
                        let alias_list: Vec<(String, String, String, Option<String>, bool)> = {
                            session.alias_manager.sorted_aliases.iter()
                                .filter_map(|name| {
                                    session.alias_manager.aliases.get(name).map(|a| {
                                        (a.name.clone(), a.pattern.clone(), a.replacement.clone(), a.category.clone(), a.enabled)
                                    })
                                })
                                .collect()
                        };
                        
                        let mut grouped_aliases: std::collections::BTreeMap<Option<String>, Vec<(String, String, String, Option<String>, bool)>> = std::collections::BTreeMap::new();
                        for item in alias_list {
                            grouped_aliases.entry(item.3.clone()).or_default().push(item);
                        }
                        
                        let mut to_delete: Option<String> = None;
                        let mut to_edit: Option<(String, String, String, String)> = None;
                        let mut to_toggle_category: Option<(Option<String>, bool)> = None;
                        
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if grouped_aliases.is_empty() {
                                ui.label("尚無別名");
                            } else {
                                for (category, items) in grouped_aliases {
                                    let category_name = category.as_deref().unwrap_or("未分類");
                                    
                                    ui.horizontal(|ui| {
                                        let all_enabled = items.iter().all(|i| i.4);
                                        let mut current_all_enabled = all_enabled;
                                        if ui.checkbox(&mut current_all_enabled, "").changed() {
                                            to_toggle_category = Some((category.clone(), current_all_enabled));
                                        }

                                        egui::CollapsingHeader::new(RichText::new(category_name).strong())
                                            .default_open(true)
                                            .show(ui, |ui| {
                                                for (name, pattern, replacement, cat, enabled) in items {
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(10.0);
                                                        let mut current_enabled = enabled;
                                                        if ui.checkbox(&mut current_enabled, "").changed() {
                                                            if let Some(alias) = session.alias_manager.aliases.get_mut(&name) {
                                                                alias.enabled = current_enabled;
                                                                needs_save = true;
                                                            }
                                                        }
                                                        ui.label(format!("{} → {}", pattern, replacement));
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.small_button("🗑️").clicked() {
                                                                to_delete = Some(name.clone());
                                                            }
                                                            if ui.small_button("✏️").clicked() {
                                                                to_edit = Some((name.clone(), pattern.clone(), replacement.clone(), cat.unwrap_or_default()));
                                                            }
                                                        });
                                                    });
                                                }
                                            });
                                    });
                                }
                            }
                        });
                        
                        if let Some((cat, enabled)) = to_toggle_category {
                            for alias in session.alias_manager.aliases.values_mut() {
                                if alias.category == cat {
                                    alias.enabled = enabled;
                                }
                            }
                            needs_save = true;
                        }
                        if let Some(name) = to_delete {
                            session.alias_manager.remove(&name);
                            needs_save = true;
                        }
                        if let Some((name, pattern, replacement, category)) = to_edit {
                            self.editing_alias_name = Some(name);
                            self.alias_edit_pattern = pattern;
                            self.alias_edit_replacement = replacement;
                            self.alias_edit_category = category;
                            self.show_alias_window = true;
                        }
                    }
                    SettingsTab::Trigger => {
                        ui.horizontal(|ui| {
                            ui.heading("觸發器管理");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("➕ 新增觸發器").clicked() {
                                    self.editing_trigger_name = Some(String::new());
                                    self.trigger_edit_name = String::new();
                                    self.trigger_edit_pattern = String::new();
                                    self.trigger_edit_action = String::new();
                                    self.trigger_edit_category = String::new();
                                    self.show_trigger_window = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        
                        let trigger_list: Vec<(String, String, String, Option<String>, bool, bool, String)> = {
                            session.trigger_manager.order.iter()
                                .filter_map(|name| {
                                    session.trigger_manager.triggers.get(name).map(|t| {
                                        let pattern_text = match &t.pattern {
                                            TriggerPattern::Contains(s) => format!("包含: {}", s),
                                            TriggerPattern::StartsWith(s) => format!("開頭: {}", s),
                                            TriggerPattern::EndsWith(s) => format!("結尾: {}", s),
                                            TriggerPattern::Regex(s) => format!("正則: {}", s),
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
                                        (t.name.clone(), pattern_text, clean_pattern, t.category.clone(), t.enabled, is_script, action_str)
                                    })
                                })
                                .collect()
                        };
                        
                        let mut grouped_triggers: std::collections::BTreeMap<Option<String>, Vec<(String, String, String, Option<String>, bool, bool, String)>> = std::collections::BTreeMap::new();
                        for item in trigger_list {
                            grouped_triggers.entry(item.3.clone()).or_default().push(item);
                        }
                        
                        let mut to_delete: Option<String> = None;
                        let mut to_edit: Option<(String, String, String, bool, String)> = None;
                        let mut to_toggle_category: Option<(Option<String>, bool)> = None;
                        
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if grouped_triggers.is_empty() {
                                ui.label("尚無觸發器");
                            } else {
                                for (category, items) in grouped_triggers {
                                    let category_name = category.as_deref().unwrap_or("未分類");

                                    ui.horizontal(|ui| {
                                        let all_enabled = items.iter().all(|i| i.4);
                                        let mut current_all_enabled = all_enabled;
                                        if ui.checkbox(&mut current_all_enabled, "").changed() {
                                            to_toggle_category = Some((category.clone(), current_all_enabled));
                                        }

                                        egui::CollapsingHeader::new(RichText::new(category_name).strong())
                                            .default_open(true)
                                            .show(ui, |ui| {
                                                for (name, pattern_text, clean_pattern, cat, enabled, is_script, action_str) in items {
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(10.0);
                                                        let mut current_enabled = enabled;
                                                        if ui.checkbox(&mut current_enabled, "").changed() {
                                                            if let Some(trigger) = session.trigger_manager.triggers.get_mut(&name) {
                                                                trigger.enabled = current_enabled;
                                                                needs_save = true;
                                                            }
                                                        }
                                                        ui.label(format!("{} [{}]", name, pattern_text));
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.small_button("🗑️").clicked() {
                                                                to_delete = Some(name.clone());
                                                            }
                                                            if ui.small_button("✏️").clicked() {
                                                                to_edit = Some((name.clone(), clean_pattern.clone(), action_str.clone(), is_script, cat.unwrap_or_default()));
                                                            }
                                                        });
                                                    });
                                                }
                                            });
                                    });
                                }
                            }
                        });
                        
                        if let Some((cat, enabled)) = to_toggle_category {
                            for trigger in session.trigger_manager.triggers.values_mut() {
                                if trigger.category == cat {
                                    trigger.enabled = enabled;
                                }
                            }
                            needs_save = true;
                        }
                        if let Some(name) = to_delete {
                            session.trigger_manager.remove(&name);
                            needs_save = true;
                        }
                        if let Some((name, pattern, action, is_script, category)) = to_edit {
                            self.editing_trigger_name = Some(name.clone());
                            self.trigger_edit_name = name;
                            self.trigger_edit_pattern = pattern;
                            self.trigger_edit_action = action;
                            self.trigger_edit_category = category;
                            self.trigger_edit_is_script = is_script;
                            self.show_trigger_window = true;
                        }
                    }
                    SettingsTab::Logger => {
                        ui.heading("日誌控制");
                        ui.add_space(10.0);
                        
                        if session.logger.is_recording() {
                            ui.label(format!("狀態: 正在記錄中 ({})", session.logger.path().map(|p| p.display().to_string()).unwrap_or_default()));
                            if ui.button("停止記錄").clicked() {
                                let _ = session.logger.stop();
                            }
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
                        
                        ui.checkbox(&mut session.auto_scroll, "自動捲動畫面");
                        ui.add_space(5.0);
                        ui.label(format!("當前補齊字典大小: {} 個單字", session.screen_words.len()));
                        ui.label("更多設定即將推出...");
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

impl eframe::App for MudApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // === 1. 背景邏輯處理 ===
        
        // 檢查自動重連
        self.check_reconnect(ctx);

        // 處理待連線的 Profile
        if let Some(profile_name) = self.pending_connect_profile.take() {
            self.connect_to_profile(&profile_name, ctx.clone());
        }

        // 處理計時器
        if let Some(session) = self.session_manager.active_session_mut() {
            session.check_timers();
        }

        // 繪製其他視窗
        // Note: profiles and settings are already handled above in the floating section
        
        let mut needs_save = false;
        if self.show_alias_window {
            Self::render_alias_edit(
                ctx,
                self.session_manager.active_session_mut(),
                &mut self.editing_alias_name,
                &mut self.alias_edit_pattern,
                &mut self.alias_edit_replacement,
                &mut self.alias_edit_category,
                &mut self.show_alias_window,
                &mut needs_save,
            );
        }
        if self.show_trigger_window {
            Self::render_trigger_edit(
                ctx,
                self.session_manager.active_session_mut(),
                &mut self.editing_trigger_name,
                &mut self.trigger_edit_name,
                &mut self.trigger_edit_pattern,
                &mut self.trigger_edit_action,
                &mut self.trigger_edit_category,
                &mut self.trigger_edit_is_script,
                &mut self.show_trigger_window,
                &mut needs_save,
            );
        }
        if needs_save {
            self.save_config();
        }

        // 處理網路訊息
        self.process_messages();

        // 設定暗黑模式
        ctx.set_visuals(egui::Visuals::dark());

        // 使用局部變數記錄
        let active_id = self.session_manager.active_id();
        let any_popup_open = self.show_settings_window || self.show_alias_window || self.show_trigger_window || self.show_profile_window;
        let active_window_id = self.active_window_id.clone();

        // 記錄待執行的延遲動作（避免在閉包中借用 self）
        let mut pending_action = None;

        // === 2. UI 渲染 ===

        // === 頂部：狀態列 + 功能鍵 ===
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            // 第一行：狀態列
            ui.horizontal(|ui| {
                if let Some(session) = self.session_manager.active_session() {
                    ui.label("伺服器:");
                    ui.label(RichText::new(&session.host).strong());
                    ui.label(":");
                    ui.label(&session.port);
                    ui.separator();

                    use crate::session::ConnectionStatus as SessionStatus;
                    match &session.status {
                        SessionStatus::Disconnected => {
                            ui.label(RichText::new("● 未連線").color(Color32::GRAY));
                        }
                        SessionStatus::Connecting => {
                            ui.spinner();
                            ui.label(RichText::new("連線中...").color(Color32::YELLOW));
                        }
                        SessionStatus::Connected(_) => {
                            ui.label(RichText::new("● 已連線").color(Color32::GREEN));
                        }
                        SessionStatus::Reconnecting => {
                            ui.spinner();
                            ui.label(RichText::new("⟳ 重連中...").color(Color32::YELLOW));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &session.status {
                            SessionStatus::Disconnected => {
                                if ui.button("🔌 連線").clicked() {
                                    pending_action = Some(PendingAction::Connect(session.id));
                                }
                            }
                            SessionStatus::Connected(_) => {
                                if ui.button("❌ 斷線").clicked() {
                                    pending_action = Some(PendingAction::Disconnect(session.id));
                                }
                            }
                            _ => {}
                        }
                    });
                } else {
                    ui.label(RichText::new("請從「連線管理」點擊連線以開始").italics().color(Color32::GRAY));
                }
            });

            ui.separator();

            // 第二行：功能鍵
            ui.horizontal(|ui| {
                if ui.button("F1 說明").clicked() {}
                if ui.button("F2 別名").clicked() { pending_action = Some(PendingAction::ToggleSettings); }
                if ui.button("F3 觸發").clicked() { pending_action = Some(PendingAction::ToggleSettings); }
                
                ui.separator();
                // 分頁列
                if self.session_manager.len() > 0 {
                    for i in 0..self.session_manager.len() {
                        let is_active = i == self.session_manager.active_index();
                        if let Some(s) = self.session_manager.sessions().get(i) {
                            if ui.selectable_label(is_active, s.tab_title()).clicked() {
                                pending_action = Some(PendingAction::SwitchTab(i));
                            }
                        }
                    }
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕").clicked() {
                        pending_action = Some(PendingAction::ToggleProfile);
                    }
                });
            });
        });

        // === 右側：工具面板 ===
        egui::SidePanel::right("tools_panel")
            .resizable(true)
            .default_width(140.0)
            .show(ctx, |ui| {
                if let Some(session) = self.session_manager.active_session() {
                    ui.heading("視窗");
                    ui.separator();

                    for window in session.window_manager.windows() {
                        let is_active = window.id == active_window_id;
                        if ui.selectable_label(is_active, &window.title).clicked() {
                            pending_action = Some(PendingAction::SwitchWindow(window.id.clone()));
                        }
                    }

                    ui.add_space(15.0);
                    ui.heading("管理");
                    ui.separator();

                    if ui.button("⚙ 設定中心").clicked() {
                        pending_action = Some(PendingAction::ToggleSettings);
                    }
                    if ui.button("👤 連線管理").clicked() {
                        pending_action = Some(PendingAction::ToggleProfile);
                    }
                } else {
                    ui.heading("管理");
                    ui.separator();
                    if ui.button("👤 連線管理").clicked() {
                        pending_action = Some(PendingAction::ToggleProfile);
                    }
                }
            });

        // === 底部：輸入區 ===
        if let Some(id) = active_id {
            egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
                if let Some(session) = self.session_manager.get_mut(id) {
                    ui.add_space(5.0);
                    Self::render_input_area(ui, session, any_popup_open);
                    ui.add_space(5.0);
                }
            });

            // === 中央：訊息區 ===
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(session) = self.session_manager.get_mut(id) {
                    Self::render_message_area(ui, session, &active_window_id);
                }
            });

            // 處理快捷鍵 (不直接傳遞 session，避免借用衝突)
            self.handle_keyboard_shortcuts(ctx, &mut pending_action);
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.heading("請點擊右上「＋」或「連線管理」按鈕選擇一個 Profile 連線。");
                });
            });
        }

        // === 動作處理 ===
        if let Some(action) = pending_action {
            match action {
                PendingAction::Connect(id) => self.start_connection(id, ctx.clone()),
                PendingAction::Disconnect(id) => {
                    if let Some(session) = self.session_manager.get_mut(id) {
                        if let Some(tx) = &session.command_tx {
                            let _ = tx.blocking_send(crate::session::Command::Disconnect);
                        }
                    }
                }
                PendingAction::SwitchTab(idx) => { self.session_manager.switch_tab(idx); }
                PendingAction::PrevTab => { self.session_manager.prev_tab(); }
                PendingAction::NextTab => { self.session_manager.next_tab(); }
                PendingAction::SwitchWindow(win_id) => { self.active_window_id = win_id; }
                PendingAction::ToggleSettings => { self.show_settings_window = !self.show_settings_window; }
                PendingAction::ToggleProfile => { self.show_profile_window = !self.show_profile_window; }
                PendingAction::ClearActiveWindow => {
                    if let Some(id) = active_id {
                        if let Some(session) = self.session_manager.get_mut(id) {
                            if let Some(window) = session.window_manager.get_mut(&active_window_id) {
                                window.clear();
                            }
                        }
                    }
                }
            }
        }

        // 彈出視窗
        if self.show_profile_window {
            self.render_profile_window(ctx);
        }
        
        // 設定視窗
        if self.show_settings_window {
            self.render_settings_window(ctx);
        }

        // 持續刷新
        ctx.request_repaint();
    }
}

/// 延階段動作
enum PendingAction {
    Connect(crate::session::SessionId),
    Disconnect(crate::session::SessionId),
    SwitchTab(usize),
    PrevTab,
    NextTab,
    SwitchWindow(String),
    ToggleSettings,
    ToggleProfile,
    ClearActiveWindow,
}


/// 簡易時間戳記（避免引入大型時間庫）
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// 清理 pattern 字串，移除可能的 Debug 格式（如 Contains("...")）
#[allow(dead_code)]
fn clean_pattern_string(pattern: &str) -> String {
    let s = pattern.trim();
    
    // 處理 Contains("...")、StartsWith("...")、EndsWith("...")、Regex("...") 格式
    for prefix in ["Contains(\"", "StartsWith(\"", "EndsWith(\"", "Regex(\""] {
        if s.starts_with(prefix) && s.ends_with("\")") {
            let inner = &s[prefix.len()..s.len() - 2];
            // 處理跳脫字元
            return inner.replace("\\\"", "\"").replace("\\\\", "\\");
        }
    }
    
    s.to_string()
}
