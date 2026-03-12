//! MUD Client 主要 UI 邏輯

mod settings;
mod sidebar;
mod profile;

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, TextEdit};
use eframe::egui::text::LayoutJob;
use mudcore::{TelnetClient, TriggerAction, TriggerPattern};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::api::{self, ApiStateManager};
use crate::config::{GlobalConfig, ProfileManager};
use crate::session::SessionManager;

/// 根據主題調整 ANSI 前景色，確保在背景上可讀
/// - 淺色主題：壓暗過亮的顏色
/// - 深色主題：提亮過暗的顏色（如深藍、深紅等裝備顏色）
fn adapt_fg_color(c: Color32, dark_mode: bool) -> Color32 {
    let [r, g, b, a] = c.to_array();
    // 感知亮度（ITU-R BT.601）
    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;

    if dark_mode {
        // 深色主題（背景 ~27）：亮度太低的前景提亮
        if luma >= 50.0 {
            return c;
        }
        if luma < 1.0 {
            return Color32::from_rgba_unmultiplied(80, 80, 80, a);
        }
        let scale = 80.0 / luma;
        Color32::from_rgba_unmultiplied(
            (r as f32 * scale).min(255.0) as u8,
            (g as f32 * scale).min(255.0) as u8,
            (b as f32 * scale).min(255.0) as u8,
            a,
        )
    } else {
        // 淺色主題（背景 ~248）：壓暗前景確保對比度
        // 閾值 60：luma 低於此值的深色不動（如紅、藍）
        if luma < 60.0 {
            return c;
        }
        // 目標 luma 50：確保與背景 248 有足夠對比（約 10:1）
        let scale = 50.0 / luma;
        Color32::from_rgba_unmultiplied(
            (r as f32 * scale) as u8,
            (g as f32 * scale) as u8,
            (b as f32 * scale) as u8,
            a,
        )
    }
}


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
    alias_edit_name: String,
    alias_edit_pattern: String,
    alias_edit_replacement: String,
    alias_edit_category: String,
    alias_edit_is_script: bool,
    alias_search_text: String,

    // === 觸發器編輯狀態 ===
    show_trigger_window: bool,
    editing_trigger_name: Option<String>,
    trigger_edit_name: String,
    trigger_edit_pattern: String,
    trigger_edit_action: String,
    trigger_edit_category: String,
    trigger_edit_is_script: bool,
    trigger_edit_pattern_type: String,
    trigger_search_text: String,

    // === 刪除確認狀態 ===
    pending_alias_delete: Option<String>,
    pending_trigger_delete: Option<String>,

    // === 路徑編輯狀態 ===
    show_path_window: bool,
    editing_path_name: Option<String>,
    path_edit_name: String,
    path_edit_value: String,
    path_edit_category: String,

    // === Profile 編輯狀態 ===
    show_profile_edit_window: bool,
    editing_profile_original_name: Option<String>,
    profile_edit_name: String,
    profile_edit_display_name: String,
    profile_edit_host: String,
    profile_edit_port: String,
    profile_edit_username: String,
    profile_edit_password: String,

    /// 設定視窗開關
    show_settings_window: bool,

    /// 設定範圍 (Global/Profile)
    settings_scope: SettingsScope,
    
    // === 側邊欄狀態 ===
    side_panel_tab: SidePanelTab,
    /// 攻略檔案列表快取 (PathBuf)
    guide_file_list: Vec<std::path::PathBuf>,
    /// 當前選中的攻略檔案內容
    active_guide_content: String,
    /// 當前選中的攻略檔案名稱
    active_guide_name: Option<String>,

    /// API 狀態管理器（每個 Session 獨立的 API 狀態）
    api_state_mgr: ApiStateManager,

    /// 地圖渲染器
    map_renderer: crate::mapper::MapRenderer,

    /// 終端管理器
    terminal_manager: crate::terminal::TerminalManager,

    /// Toast 訊息 (文字, 建立時間)
    toast_message: Option<(String, Instant)>,

    /// Debug panel: Lua console input
    debug_lua_input: String,
    /// Debug panel: last Lua result
    debug_lua_result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsScope {
    Profile,
    Global,
}

/// 設定中心標籤頁
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Alias,
    Trigger,
    Path,
    Logger,
    General,
}

/// 側邊欄標籤頁
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidePanelTab {
    Tools,
    Guide,
    Notes,
    Map,
    Terminal,
    Debug,
}

impl MudApp {
    /// 創建新的 MUD 客戶端應用程式
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 壓縮超過 7 天的舊日誌
        mudcore::logger::compress_old_logs("logs", 7);

        // 載入設定並設定字型
        let global_config = GlobalConfig::load();
        Self::setup_fonts(&cc.egui_ctx, &global_config.ui.font_family);

        // 創建 Tokio 運行時
        let runtime = Runtime::new().expect("無法創建 Tokio 運行時");

        // 建立並啟動 API Server（多 Session 版）
        let api_state_mgr = ApiStateManager::new();
        api::start_api_server(&runtime, api_state_mgr.clone());

        Self {
            runtime,
            settings_tab: SettingsTab::Alias,
            // 多帳號系統
            profile_manager: ProfileManager::new(),
            session_manager: SessionManager::new(),
            global_config,
            show_profile_window: false,
            pending_connect_profile: None,

            // UI 狀態
            active_window_id: "main".to_string(),
            show_alias_window: false,
            editing_alias_name: None,
            alias_edit_name: String::new(),
            alias_edit_pattern: String::new(),
            alias_edit_replacement: String::new(),
            alias_edit_category: String::new(),
            alias_edit_is_script: false,
            show_trigger_window: false,
            editing_trigger_name: None,
            trigger_edit_name: String::new(),
            trigger_edit_pattern: String::new(),
            trigger_edit_action: String::new(),
            trigger_edit_category: String::new(),
            trigger_edit_is_script: false,
            trigger_edit_pattern_type: "auto".to_string(),
            pending_alias_delete: None,
            pending_trigger_delete: None,
            
            // 路徑狀態
            show_path_window: false,
            editing_path_name: None,
            path_edit_name: String::new(),
            path_edit_value: String::new(),
            path_edit_category: String::new(),
            
            // Profile 編輯狀態初始化
            show_profile_edit_window: false,
            editing_profile_original_name: None,
            profile_edit_name: String::new(),
            profile_edit_display_name: String::new(),
            profile_edit_host: String::new(),
            profile_edit_port: String::new(),
            profile_edit_username: String::new(),
            profile_edit_password: String::new(),

            show_settings_window: false,
            settings_scope: SettingsScope::Profile,
            alias_search_text: String::new(),
            trigger_search_text: String::new(),
            
            side_panel_tab: SidePanelTab::Tools,
            guide_file_list: Vec::new(),
            active_guide_content: String::new(),
            active_guide_name: None,

            api_state_mgr,

            map_renderer: crate::mapper::MapRenderer::default(),

            terminal_manager: crate::terminal::TerminalManager::new(),

            toast_message: None,

            debug_lua_input: String::new(),
            debug_lua_result: None,
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
                    // 檢查是否與全域設定相同 (完全相同則不儲存，實現 Clean Save)
                    let is_global_identical = self.global_config.global_aliases.iter().any(|ga| {
                        ga.name == a.name && 
                        ga.pattern == a.pattern && 
                        ga.replacement == a.replacement && 
                        ga.is_script == a.is_script &&
                        ga.enabled == a.enabled &&
                        ga.category == a.category
                    });

                    if !is_global_identical {
                        new_aliases.push(crate::config::AliasConfig {
                            name: a.name.clone(),
                            pattern: a.pattern.clone(),
                            replacement: a.replacement.clone(),
                            category: a.category.clone(),
                            is_script: a.is_script,
                            enabled: a.enabled,
                        });
                    }
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
                     
                     // 檢查是否與全域設定相同
                     let is_global_identical = self.global_config.global_triggers.iter().any(|gt| {
                         gt.name == t.name && 
                         gt.pattern == pat_str && 
                         gt.action == action_str && 
                         gt.is_script == is_script &&
                         gt.enabled == t.enabled &&
                         gt.category == t.category
                     });

                     if !is_global_identical {
                         let pattern_type = match &t.pattern {
                             TriggerPattern::Contains(_) => Some("contains".to_string()),
                             TriggerPattern::StartsWith(_) => Some("startswith".to_string()),
                             TriggerPattern::EndsWith(_) => Some("endswith".to_string()),
                             TriggerPattern::Regex(_) => Some("regex".to_string()),
                         };
                         new_triggers.push(crate::config::TriggerConfig {
                             name: t.name.clone(),
                             pattern: pat_str,
                             action: action_str,
                             category: t.category.clone(),
                             is_script,
                             enabled: t.enabled,
                             pattern_type,
                             group: t.group.clone(),
                         });
                     }
                 }
             }

             // 3. 同步 Path
             let mut new_paths = Vec::new();
             for name in &session.path_manager.sorted_keys {
                 if let Some(p) = session.path_manager.get(name) {
                     new_paths.push(crate::config::PathConfig {
                         name: p.name.clone(),
                         value: p.value.clone(),
                         category: p.category.clone(),
                     });
                 }
             }

             // 4. 更新 ProfileManager 並儲存
              if let Some(profile) = self.profile_manager.get_mut(&profile_name) {
                  profile.aliases = new_aliases;
                  profile.triggers = new_triggers;
                  profile.paths = new_paths;
                  profile.notes = session.notes.clone();
                  
                  // 儲存到磁碟
                 let p = profile.clone();
                 if let Err(e) = self.profile_manager.save(p) {
                     tracing::error!("Failed to save profile {}: {}", profile_name, e);
                 } else {
                     tracing::info!("Saved profile: {}", profile_name);
                 }
             }
        }

        // 儲存所有 Session 的指令字典
        for session in self.session_manager.sessions() {
            session.command_dict.save(std::path::Path::new("data/command_dict.json"));
        }
        
        // 儲存全域設定
        if let Err(e) = self.global_config.save() {
            tracing::error!("Failed to save global config: {}", e);
        }
    }

    /// 初始化字型設定（支援自訂字型家族）
    fn setup_fonts(ctx: &egui::Context, font_family: &str) {
        let mut fonts = egui::FontDefinitions::default();

        // 嘗試載入使用者指定的系統字型
        let custom_loaded = if !font_family.is_empty() {
            Self::try_load_system_font(&mut fonts, font_family)
        } else {
            false
        };

        if !custom_loaded {
            // 使用內建 Sarasa Mono TC
            let reg_bytes = include_bytes!("../../assets/fonts/SarasaMonoTC-Regular.ttf");
            let bold_bytes = include_bytes!("../../assets/fonts/SarasaMonoTC-Bold.ttf");

            fonts.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(reg_bytes.to_vec())),
            );
            fonts.font_data.insert(
                "cjk_bold".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bold_bytes.to_vec())),
            );
            tracing::info!("字型載入: 內建 SarasaMonoTC");
        }

        // 設定字型家族優先順序
        fonts.families.get_mut(&egui::FontFamily::Monospace)
            .map(|f| f.insert(0, "cjk".to_owned()));
        fonts.families.get_mut(&egui::FontFamily::Proportional)
            .map(|f| f.insert(0, "cjk".to_owned()));
        fonts.families.insert(
            egui::FontFamily::Name("cjk_bold".into()),
            vec!["cjk_bold".to_owned()],
        );

        ctx.set_fonts(fonts);
    }

    /// 嘗試從系統載入指定字型（Regular + Bold）
    fn try_load_system_font(fonts: &mut egui::FontDefinitions, family_name: &str) -> bool {
        use font_kit::source::SystemSource;
        use font_kit::properties::{Properties, Weight};
        use font_kit::family_name::FamilyName;

        let source = SystemSource::new();
        let family = [FamilyName::Title(family_name.to_string())];

        // 載入 Regular
        let reg_data = source.select_best_match(&family, &Properties::new())
            .ok().and_then(|h| h.load().ok())
            .and_then(|f| f.copy_font_data());

        let Some(reg_data) = reg_data else {
            tracing::warn!("無法載入系統字型: {}", family_name);
            return false;
        };

        // Arc::try_unwrap 避免多餘的 Vec clone
        let unwrap_arc = |arc: std::sync::Arc<Vec<u8>>| -> Vec<u8> {
            std::sync::Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone())
        };

        fonts.font_data.insert(
            "cjk".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(unwrap_arc(reg_data))),
        );

        // 載入 Bold（失敗則以 Regular 替代）
        let mut bold_props = Properties::new();
        bold_props.weight = Weight::BOLD;
        let bold_data = source.select_best_match(&family, &bold_props)
            .ok().and_then(|h| h.load().ok())
            .and_then(|f| f.copy_font_data());

        if let Some(data) = bold_data {
            fonts.font_data.insert(
                "cjk_bold".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(unwrap_arc(data))),
            );
        } else if let Some(reg) = fonts.font_data.get("cjk") {
            fonts.font_data.insert("cjk_bold".to_owned(), reg.clone());
        }

        tracing::info!("字型載入: 系統字型 {}", family_name);
        true
    }

    /// 列出系統可用的等寬字型（含 CJK 支援檢測）
    fn list_monospace_fonts() -> Vec<String> {
        use font_kit::source::SystemSource;
        let source = SystemSource::new();
        let all = match source.all_families() {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        // 過濾出可能的等寬/CJK 字型（啟發式：名稱包含 mono/code/courier/consol 等關鍵字，或 CJK 字型）
        let mono_keywords = ["mono", "code", "courier", "consol", "menlo", "fira", "hack",
                             "iosevka", "jetbrains", "source han", "noto sans mono",
                             "sarasa", "cascadia", "inconsolata", "dejavu sans mono",
                             "liberation mono", "ubuntu mono", "sf mono", "andale",
                             "等寬", "黑體", "gothic", "ming", "宋", "微軟正黑",
                             "pingfang", "hiragino"];
        let mut result: Vec<String> = all.into_iter()
            .filter(|name| {
                let lower = name.to_lowercase();
                mono_keywords.iter().any(|kw| lower.contains(kw))
            })
            .collect();
        result.sort();
        result.dedup();
        result
    }

    /// 從 Profile 建立連線
    fn connect_to_profile(&mut self, profile_name: &str, ctx: egui::Context) {
        // 從 ProfileManager 取得 Profile
        if let Some(profile) = self.profile_manager.get(profile_name) {
            tracing::info!("建立 Profile 連線: {}", profile_name);
            
            // 為新 Session 建立專屬的 ApiState
            let session_id = self.session_manager.create_session(profile, &self.api_state_mgr);
            
            // 啟動連線
            self.start_connection(session_id, ctx.clone());
            
            // 顯示本地訊息
            if let Some(session) = self.session_manager.get_mut(session_id) {
                session.handle_text(&format!(">>> 已建立 Profile 會話: {} ({}:{})\n", profile_name, session.host, session.port), true);
            }
        } else {
            tracing::warn!("找不到 Profile: {}", profile_name);
        }
    }



    /// 啟動指定 Session 的網路連線
    fn start_connection(&mut self, session_id: crate::session::SessionId, ctx: egui::Context) {
        let (host, port, username, password) = {
            let session = match self.session_manager.get(session_id) {
                Some(s) => s,
                None => return,
            };
            (
                session.host.clone(), 
                session.port.parse::<u16>().unwrap_or(7777),
                session.username.clone(),
                session.password.clone(),
            )
        };

        // 創建 channels
        use crate::session::Command as SessionCommand;
        use crate::session::NetMessage;
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<SessionCommand>(32);
        let (msg_tx, msg_rx) = mpsc::channel::<NetMessage>(1024);

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
                            SessionCommand::Connect(h, p, u, pwd) => {
                                match client.connect(&h, p).await {
                                    Ok(_) => {
                                        let _ = msg_tx.send(NetMessage::Text(format!(">>> 已連線到 {}:{}\n", h, p), Vec::new())).await;

                                        // 自動登入邏輯
                                        if let Some(username) = u {
                                            // 稍微延遲一點點確保連線穩定（簡易版）
                                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                            if let Err(e) = client.send(&username).await {
                                                let _ = msg_tx.send(NetMessage::Text(format!(">>> 自動登入(帳號)失敗: {}\n", e), Vec::new())).await;
                                            } else {
                                                // let _ = msg_tx.send(">>> 已發送帳號\n".to_string()).await;
                                            }

                                            if let Some(password) = pwd {
                                                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                                if let Err(e) = client.send(&password).await {
                                                     let _ = msg_tx.send(NetMessage::Text(format!(">>> 自動登入(密碼)失敗: {}\n", e), Vec::new())).await;
                                                } else {
                                                    let _ = msg_tx.send(NetMessage::Text(">>> 已嘗試自動登入\n".to_string(), Vec::new())).await;
                                                    
                                                    // 延遲並發送空指令，確保能排除可能殘留在伺服器輸入緩衝區的任何干擾
                                                    // 增加延遲到 1000ms 確保伺服器已完全進入遊戲狀態
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                                    let _ = client.send("").await;
                                                }
                                            }
                                        }

                                        // 開始讀取迴圈
                                        loop {
                                            tokio::select! {
                                                result = client.read_with_widths() => {
                                                    match result {
                                                        Ok((text, widths)) if !text.is_empty() => {
                                                            let _ = msg_tx.send(NetMessage::Text(text, widths)).await;
                                                            ctx.request_repaint();
                                                        }
                                                        Ok(_) => {
                                                            let _ = msg_tx.send(NetMessage::Text(">>> 連線已關閉\n".to_string(), Vec::new())).await;
                                                            break;
                                                        }
                                                        Err(e) => {
                                                            let _ = msg_tx.send(NetMessage::Text(format!(">>> 連線已關閉 (錯誤: {})\n", e), Vec::new())).await;
                                                            break;
                                                        }
                                                    }
                                                }
                                                Some(cmd) = cmd_rx.recv() => {
                                                    match cmd {
                                                        SessionCommand::Send(text) => {
                                                            if let Err(e) = client.send(&text).await {
                                                                let _ = msg_tx.send(NetMessage::Text(format!(">>> 發送失敗: {}\n", e), Vec::new())).await;
                                                            }
                                                        }
                                                        SessionCommand::CollectResponse { command, callback } => {
                                                            // 處理 CollectResponse 佇列（支援 send_chain 連續多個）
                                                            let mut queue: Vec<(String, Option<mudcore::script::LuaCallback>)> = vec![(command, callback)];

                                                            // === Phase 0: 收集 channel 中所有待處理的指令 ===
                                                            // Send 立即發出，CollectResponse 加入佇列依序處理
                                                            while let Ok(pending_cmd) = cmd_rx.try_recv() {
                                                                match pending_cmd {
                                                                    SessionCommand::Send(text) => {
                                                                        if let Err(e) = client.send(&text).await {
                                                                            let _ = msg_tx.send(NetMessage::Text(format!(">>> 發送失敗: {}\n", e), Vec::new())).await;
                                                                        }
                                                                    }
                                                                    SessionCommand::CollectResponse { command: c, callback: cb } => {
                                                                        queue.push((c, cb));
                                                                    }
                                                                    SessionCommand::Disconnect => {
                                                                        client.disconnect().await;
                                                                        let _ = msg_tx.send(NetMessage::Text(">>> 已斷開連線\n".to_string(), Vec::new())).await;
                                                                        queue.clear();
                                                                        break;
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }

                                                            // === 依序處理每個 CollectResponse ===
                                                            for (cmd, cb) in queue {
                                                                // Drain — 讀盡管線中所有待處理的回應
                                                                loop {
                                                                    match tokio::time::timeout(
                                                                        std::time::Duration::from_millis(500),
                                                                        client.read_with_widths()
                                                                    ).await {
                                                                        Ok(Ok((text, widths))) if !text.is_empty() => {
                                                                            let _ = msg_tx.send(NetMessage::Text(text, widths)).await;
                                                                            ctx.request_repaint();
                                                                        }
                                                                        _ => break,
                                                                    }
                                                                }

                                                                // 發送指令
                                                                if let Err(e) = client.send(&cmd).await {
                                                                    let _ = msg_tx.send(NetMessage::Text(format!(">>> CollectResponse 發送失敗: {}\n", e), Vec::new())).await;
                                                                    continue;
                                                                }

                                                                // Collect — 收集回應直到 prompt
                                                                let mut collected_lines: Vec<String> = Vec::new();
                                                                loop {
                                                                    match tokio::time::timeout(
                                                                        std::time::Duration::from_secs(5),
                                                                        client.read_with_widths()
                                                                    ).await {
                                                                        Ok(Ok((text, widths))) if !text.is_empty() => {
                                                                            let _ = msg_tx.send(NetMessage::Text(text.clone(), widths)).await;
                                                                            ctx.request_repaint();

                                                                            for line in text.split('\n') {
                                                                                let clean = crate::ansi::strip_ansi(line);
                                                                                let trimmed = clean.replace('\r', "");
                                                                                let trimmed = trimmed.trim();
                                                                                if !trimmed.is_empty() {
                                                                                    collected_lines.push(trimmed.to_string());
                                                                                }
                                                                            }

                                                                            if !text.ends_with('\n') {
                                                                                match tokio::time::timeout(
                                                                                    std::time::Duration::from_millis(50),
                                                                                    client.read_with_widths()
                                                                                ).await {
                                                                                    Err(_) => {
                                                                                        collected_lines.pop();
                                                                                        break;
                                                                                    }
                                                                                    Ok(Ok((more, w))) if !more.is_empty() => {
                                                                                        let _ = msg_tx.send(NetMessage::Text(more.clone(), w)).await;
                                                                                        ctx.request_repaint();
                                                                                        for line in more.split('\n') {
                                                                                            let clean = crate::ansi::strip_ansi(line);
                                                                                            let trimmed = clean.replace('\r', "");
                                                                                            let trimmed = trimmed.trim();
                                                                                            if !trimmed.is_empty() {
                                                                                                collected_lines.push(trimmed.to_string());
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    _ => break,
                                                                                }
                                                                            }
                                                                        }
                                                                        _ => break,
                                                                    }
                                                                }

                                                                // 發送收集結果
                                                                let _ = msg_tx.send(NetMessage::CollectedResponse {
                                                                    lines: collected_lines,
                                                                    callback: cb,
                                                                }).await;
                                                                ctx.request_repaint();
                                                            }
                                                        }
                                                        SessionCommand::Disconnect => {
                                                            client.disconnect().await;
                                                            let _ = msg_tx.send(NetMessage::Text(">>> 已斷開連線\n".to_string(), Vec::new())).await;
                                                            break;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = msg_tx.send(NetMessage::Text(format!(">>> 連線已關閉 (連線失敗: {})\n", e), Vec::new())).await;
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
        let _ = cmd_tx.blocking_send(SessionCommand::Connect(host, port, username, password));
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
                        // 每秒刷新一次以更新倒數顯示，避免 CPU 空轉
                        ctx.request_repaint_after(std::time::Duration::from_secs(1));
                    }
                }
            }
        }
        
        for id in to_reconnect {
            self.start_connection(id, ctx.clone());
        }
    }

    /// 處理所有活躍 Session 的網路訊息
    /// 處理所有活躍 Session 的網路訊息，回傳是否有收到訊息
    fn process_messages(&mut self) -> bool {
        use crate::session::NetMessage;
        let session_ids: Vec<_> = self.session_manager.sessions().iter().map(|s| s.id).collect();
        let mut had_messages = false;

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
                had_messages = true;
                if let Some(session) = self.session_manager.get_mut(id) {
                    for msg in messages {
                        match msg {
                            NetMessage::Text(text, widths) => {
                                if widths.is_empty() {
                                    session.handle_text(&text, false);
                                } else {
                                    session.handle_text_with_widths(&text, false, Some(&widths));
                                }

                                use crate::session::ConnectionStatus as SessionStatus;
                                if text.contains("已連線到") {
                                    let info = text.replace(">>> 已連線到 ", "").replace("\n", "");
                                    session.status = SessionStatus::Connected(info.clone());
                                    session.connected_at = Some(Instant::now());
                                    session.emit_event("connected", Some(format!(r#"{{"server":"{}"}}"#, info)));
                                    if let Ok(mut api) = session.api_state.lock() {
                                        api.connection_status = format!("connected:{}", info);
                                    }
                                } else if text.contains("連線已關閉") || text.contains("已斷開連線") {
                                    session.connected_at = None;
                                    if session.auto_reconnect {
                                        use std::time::Duration;
                                        session.reconnect_delay_until = Some(Instant::now() + Duration::from_secs(self.global_config.ui.reconnect_delay_secs));
                                        session.status = SessionStatus::Reconnecting;
                                        if let Ok(mut api) = session.api_state.lock() {
                                            api.connection_status = "reconnecting".to_string();
                                        }
                                    } else {
                                        session.status = SessionStatus::Disconnected;
                                        session.emit_event("disconnected", None);
                                        if let Ok(mut api) = session.api_state.lock() {
                                            api.connection_status = "disconnected".to_string();
                                        }
                                    }
                                }
                            }
                            NetMessage::CollectedResponse { lines, callback } => {
                                session.execute_collected_response(lines, callback);
                            }
                        }
                    }
                }
            }
        }
        // 處理 API 命令
        self.process_api_commands();
        had_messages
    }

    /// 處理 API 傳入的指令和 Lua 程式碼（多 Session 版）
    fn process_api_commands(&mut self) {
        // 遍歷所有 Session 的 ApiState，取出各自的 pending 佇列
        let all_states = self.api_state_mgr.all_states();
        
        for (session_key, api_state) in all_states {
            let (commands, lua_codes, eval_lua_codes, api_queries, llm_fn_results) = {
                if let Ok(mut api) = api_state.lock() {
                    (api.drain_commands(), api.drain_lua(), api.drain_eval_lua(), api.drain_api_queries(), api.drain_llm_fn_results())
                } else {
                    continue;
                }
            };

            // 跳過沒有待處理項目的 Session
            if commands.is_empty() && lua_codes.is_empty() && eval_lua_codes.is_empty() && api_queries.is_empty() && llm_fn_results.is_empty() {
                continue;
            }

            // 找到對應的 Session 並執行（直接用數值比較，避免 String 分配）
            let session_id_num: u64 = match session_key.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let session = self.session_manager.sessions_mut().iter_mut()
                .find(|s| s.id.value() == session_id_num);
            
            if let Some(session) = session {
                for cmd in commands {
                    session.handle_user_input(&cmd);
                }
                for code in lua_codes {
                    match session.script_engine.execute_inline(&code, "API", &[], false) {
                        Ok(ctx) => session.apply_script_context(ctx),
                        Err(e) => {
                            tracing::error!("API Lua error (session {}): {}", session_key, e);
                            session.system_message(&format!("API Lua Error: {}", e));
                        }
                    }
                }
                for (code, tx) in eval_lua_codes {
                    let result_str = match session.script_engine.execute_inline_with_result(&code, "API_EVAL", &[]) {
                        Ok((ctx, result)) => {
                            session.apply_script_context(ctx);
                            result
                        }
                        Err(e) => {
                            tracing::error!("API Eval Lua error (session {}): {}", session_key, e);
                            format!("Error: {}", e)
                        }
                    };
                    let _ = tx.send(result_str);
                }
                for (query, tx) in api_queries {
                    let result = session.handle_api_query(query);
                    let _ = tx.send(result);
                }
                // LLM function callback 結果
                for (callback, result_text) in llm_fn_results {
                    if let mudcore::script::LuaCallback::Function(key) = callback {
                        match session.script_engine.execute_lua_callback_with_result(key, &result_text) {
                            Ok(ctx) => session.apply_script_context(ctx),
                            Err(e) => {
                                tracing::error!("LLM function callback error (session {}): {}", session_key, e);
                                session.system_message(&format!("[ask_llm] function callback 錯誤: {}", e));
                            }
                        }
                    }
                }
            }
        }
    }

    /// 繪製訊息顯示區（支援 ANSI 顏色）
    fn render_message_area(ui: &mut egui::Ui, session: &mut crate::session::Session, active_window_id: &str, font_size: f32, enable_blink: bool) {
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
                // font_size 由呼叫端傳入（來自 global_config.ui.font_size）
                let font_id = FontId::monospace(font_size);
                let bold_font_id = FontId::new(font_size, egui::FontFamily::Name("cjk_bold".into()));
                let dark_mode = ui.visuals().dark_mode;

                // 穩定測量：使用空格寬度作為 Mono 單元格寬度基準
                let cell_w = ui.fonts(|f| f.glyph_width(&font_id, ' '));

                let mut main_job = LayoutJob::default();
                let mut overlay_job = LayoutJob::default();
                main_job.wrap.max_width = f32::INFINITY;
                overlay_job.wrap.max_width = f32::INFINITY;
                
                let mut section_color_map = std::collections::HashMap::new();
                let mut section_font_map = std::collections::HashMap::new();
                let mut section_fg_colors: Vec<Color32> = Vec::new(); // 記錄每個 section 的前景色
                let mut has_dual_color = false;
                let mut has_blink = false;
                const BLINK_PERIOD: f64 = 1.0; // 閃爍週期（秒）
                const BLINK_DUTY: f64 = 0.5; // 亮的比例
                let blink_phase = ui.input(|i| i.time) % BLINK_PERIOD;
                let blink_visible = !enable_blink || blink_phase < BLINK_PERIOD * BLINK_DUTY;
                let mut pending_trailing_space: f32 = 0.0; // 用於置中對齊：記錄上一個字元的後半部間距
                // 字型寬度快取 — 避免每個字元都查字型系統
                let mut glyph_cache: std::collections::HashMap<(char, bool), f32> = std::collections::HashMap::new();
                let mut char_buf = String::with_capacity(4); // 重用 buffer 避免每字元分配

                if let Some(window) = session.window_manager.get(active_window_id) {
                    // 只渲染最近 N 條訊息以避免效能問題
                    let visible_lines = ((available_height / (font_size + 4.0)) as usize * 3).max(200);
                    let (slice_a, slice_b) = window.last_n_slices(visible_lines);
                    for msg in slice_a.iter().chain(slice_b.iter()) {
                        use crate::ansi::parse_ansi_with_widths;
                        let spans = parse_ansi_with_widths(&msg.content, Some(&msg.byte_widths));

                         
                        for span in spans {
                            let is_blink = span.blink;
                            if is_blink { has_blink = true; }
                            let blink_color = |c: Color32| -> Color32 {
                                if is_blink && !blink_visible { Color32::TRANSPARENT } else { c }
                            };
                            let background = span.bg_color.unwrap_or(Color32::TRANSPARENT);
                            let mut current_font_id = font_id.clone();
                            
                            let (render_color, _) = if span.bold {
                                let [r, g, b, a] = span.fg_color.to_array();
                                let bright_color = Color32::from_rgba_unmultiplied(
                                    r.saturating_add(30),
                                    g.saturating_add(30),
                                    b.saturating_add(30),
                                    a
                                );
                                current_font_id = bold_font_id.clone();
                                (bright_color, true)
                            } else {
                                (span.fg_color, false)
                            };
                            // 淺色主題：將過亮的前景色調暗以確保可讀性
                            let render_color = adapt_fg_color(render_color, dark_mode);
                            let render_color = blink_color(render_color);
                            // 判斷是否為真正的雙色字：fg_color_left 有值且 span 只有一個可見字元
                            // 多字元 span（如「紅龍護符」）→ 非雙色字，用 fg_color_left 為統一顏色
                            let visible_chars = span.text.chars().filter(|c| *c != '\n' && *c != '\r').count();
                            let is_real_dual_color = span.fg_color_left.is_some() && visible_chars == 1;
                            
                            // 非雙色字渲染
                            if !is_real_dual_color {
                                for (idx, ch) in span.text.chars().enumerate() {
                                    if ch == '\n' || ch == '\r' {
                                        let fmt = egui::TextFormat { font_id: current_font_id.clone(), color: render_color, background, italics: false, line_height: Some(font_size + 4.0), ..Default::default() };
                                        section_fg_colors.push(render_color);
                                        char_buf.clear(); char_buf.push(ch);
                                        main_job.append(&char_buf, pending_trailing_space, fmt.clone());
                                        overlay_job.append(&char_buf, pending_trailing_space, egui::TextFormat { color: Color32::TRANSPARENT, background: Color32::TRANSPARENT, ..fmt });
                                        pending_trailing_space = 0.0;
                                        continue;
                                    }
                                    let u_w = if let Some(bw) = span.byte_widths.get(idx).copied() {
                                        bw as usize
                                    } else {
                                        if ch.is_ascii() || ch == '|' { 1 }
                                        else if ch == '\u{2103}' || ch == '\u{00a7}' { 2 }
                                        else {
                                            use unicode_width::UnicodeWidthChar;
                                            ch.width().unwrap_or(1).max(1)
                                        }
                                    };

                                    // CJK 終端環境：框線繪圖字元始終佔 2 列寬
                                    let u_w = if ch >= '\u{2500}' && ch <= '\u{259f}' { u_w.max(2) } else { u_w };
                                    let target_w = (u_w as f32) * cell_w;
                                    let actual_w = *glyph_cache.entry((ch, span.bold)).or_insert_with(|| {
                                        ui.fonts(|f| f.glyph_width(&current_font_id, ch))
                                    });
                                    
                                    // 置中對齊策略：
                                    // 1. 框線字元 (\u2500-\u259f) 或原本就佔滿 2 單元的 CJK：不置中，維持靠左以確保接縫對齊
                                    // 2. 窄字元 (如 §, \u2103) 但宣告為 2 單元寬：置中補位
                                    let extra = (if actual_w <= 0.0 { target_w } else { target_w - actual_w }).max(0.0);
                                    let is_box_or_full_cjk = (ch >= '\u{2500}' && ch <= '\u{259f}') || (u_w >= 2 && actual_w >= target_w * 0.9);
                                    
                                    let (current_leading, next_trailing) = if is_box_or_full_cjk {
                                        (extra + pending_trailing_space, 0.0)
                                    } else {
                                        (extra / 2.0 + pending_trailing_space, extra / 2.0)
                                    };
                                    pending_trailing_space = next_trailing;

                                    // 多字元 span 有 fg_color_left：CJK 字元用 fg_color_left，ASCII 用 render_color
                                    let char_color = blink_color(if let Some(lc) = span.fg_color_left {
                                        if !ch.is_ascii() { adapt_fg_color(lc, dark_mode) } else { render_color }
                                    } else {
                                        render_color
                                    });
                                    let glyph_color = if ch >= '\u{2500}' && ch <= '\u{259f}' {
                                        Color32::TRANSPARENT
                                    } else {
                                        char_color
                                    };
                                    let fmt = egui::TextFormat {
                                        font_id: current_font_id.clone(),
                                        color: glyph_color,
                                        background,
                                        italics: false,
                                        line_height: Some(font_size + 4.0),
                                        ..Default::default()
                                    };
                                    section_fg_colors.push(char_color);
                                    char_buf.clear(); char_buf.push(ch);
                                    main_job.append(&char_buf, current_leading, fmt.clone());
                                    overlay_job.append(&char_buf, current_leading, egui::TextFormat { color: Color32::TRANSPARENT, background: Color32::TRANSPARENT, ..fmt });
                                }
                                continue;
                            }

                            // 雙色字逐字元網格對齊模式
                            for (idx, ch) in span.text.chars().enumerate() {
                                if ch == '\n' || ch == '\r' {
                                    let fmt = egui::TextFormat { font_id: current_font_id.clone(), color: render_color, background, italics: false, line_height: Some(font_size + 4.0), ..Default::default() };
                                    section_fg_colors.push(render_color);
                                    char_buf.clear(); char_buf.push(ch);
                                    main_job.append(&char_buf, pending_trailing_space, fmt.clone());
                                    overlay_job.append(&char_buf, pending_trailing_space, egui::TextFormat { color: Color32::TRANSPARENT, background: Color32::TRANSPARENT, ..fmt });
                                    pending_trailing_space = 0.0;
                                    continue;
                                }

                                let u_w = if let Some(bw) = span.byte_widths.get(idx).copied() {
                                    bw as usize
                                } else {
                                    if ch.is_ascii() || ch == '|' { 1 }
                                    else if ch == '\u{2103}' || ch == '\u{00a7}' { 2 }
                                    else {
                                        use unicode_width::UnicodeWidthChar;
                                        ch.width().unwrap_or(1).max(1)
                                    }
                                };

                                // CJK 終端環境：框線繪圖字元始終佔 2 列寬
                                let u_w = if ch >= '\u{2500}' && ch <= '\u{259f}' { u_w.max(2) } else { u_w };
                                let target_w = (u_w as f32) * cell_w;
                                let actual_w = *glyph_cache.entry((ch, span.bold)).or_insert_with(|| {
                                    ui.fonts(|f| f.glyph_width(&current_font_id, ch))
                                });
                                
                                let extra = (if actual_w <= 0.0 { target_w } else { target_w - actual_w }).max(0.0);
                                let is_box_or_full_cjk = (ch >= '\u{2500}' && ch <= '\u{259f}') || (u_w >= 2 && actual_w >= target_w * 0.9);
                                
                                let (current_leading, next_trailing) = if is_box_or_full_cjk {
                                    (extra + pending_trailing_space, 0.0)
                                } else {
                                    (extra / 2.0 + pending_trailing_space, extra / 2.0)
                                };
                                pending_trailing_space = next_trailing;

                                let mut format = egui::TextFormat {
                                    font_id: current_font_id.clone(),
                                    color: render_color,
                                    background,
                                    italics: false,
                                    line_height: Some(font_size + 4.0),
                                    ..Default::default()
                                };

                                let section_idx = main_job.sections.len();
                                if let Some(lc) = span.fg_color_left {
                                    has_dual_color = true;
                                    let left_color = blink_color(adapt_fg_color(lc, dark_mode));
                                    section_color_map.insert(section_idx, (left_color, render_color));
                                    section_font_map.insert(section_idx, current_font_id.clone());
                                    
                                    let mut overlay_fmt = format.clone();
                                    format.color = Color32::TRANSPARENT;
                                    overlay_fmt.color = Color32::WHITE;
                                    overlay_fmt.background = Color32::TRANSPARENT;
                                    
                                    section_fg_colors.push(render_color);
                                    char_buf.clear(); char_buf.push(ch);
                                    main_job.append(&char_buf, current_leading, format);
                                    overlay_job.append(&char_buf, current_leading, overlay_fmt);
                                } else {
                                    let mut overlay_fmt = format.clone();
                                    overlay_fmt.color = Color32::TRANSPARENT;
                                    overlay_fmt.background = Color32::TRANSPARENT;

                                    section_fg_colors.push(render_color);
                                    char_buf.clear(); char_buf.push(ch);
                                    main_job.append(&char_buf, current_leading, format);
                                    overlay_job.append(&char_buf, current_leading, overlay_fmt);
                                }
                            }
                        }

                        // 確保訊息之間有換行，並重置置中間距
                        // 空訊息保留為空行（例如 prompt 前的空行），非空訊息補換行
                        let nl_leading = if msg.content.is_empty() && !main_job.text.is_empty() {
                            Some(0.0)
                        } else if !main_job.text.is_empty() && !main_job.text.ends_with('\n') {
                            Some(pending_trailing_space)
                        } else {
                            None
                        };
                        if let Some(leading) = nl_leading {
                            let nl_fmt = egui::TextFormat { font_id: font_id.clone(), line_height: Some(font_size + 4.0), ..Default::default() };
                            section_fg_colors.push(Color32::TRANSPARENT);
                            main_job.append("\n", leading, nl_fmt.clone());
                            overlay_job.append("\n", leading, egui::TextFormat { color: Color32::TRANSPARENT, ..nl_fmt });
                            pending_trailing_space = 0.0;
                        }
                    }
                }
                
                // 使用可選取的 Label 支援文字選取（Cmd+C 複製）
                let main_galley = ui.fonts(|f| f.layout_job(main_job.clone()));
                let label_response = ui.add(
                    egui::Label::new(egui::WidgetText::LayoutJob(main_job))
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Extend)
                );
                let rect = label_response.rect;
                
                // 右鍵選單：複製全文
                label_response.context_menu(|ui| {
                    if ui.button("複製全文").clicked() {
                        let mut all_text = String::new();
                        if let Some(window) = session.window_manager.get(active_window_id) {
                            for msg in window.messages() {
                                use crate::ansi::parse_ansi_with_widths;
                                let spans = parse_ansi_with_widths(&msg.content, Some(&msg.byte_widths));
                                for span in &spans {
                                    all_text.push_str(&span.text);
                                }
                                if !all_text.ends_with('\n') {
                                    all_text.push('\n');
                                }
                            }
                        }
                        ui.output_mut(|o| o.copied_text = all_text);
                        ui.close_menu();
                    }
                });
                    
                    // 2x 字型純文字渲染框線字元（取代幾何線段）
                    let painter = ui.painter();
                    let box_font = FontId::monospace(font_size * 2.0);
                    for row in &main_galley.rows {
                        for glyph in &row.glyphs {
                            let ch = glyph.chr;
                            if ch < '\u{2500}' || ch > '\u{259f}' { continue; }
                            
                            let fg_color = section_fg_colors.get(glyph.section_index as usize)
                                .copied().unwrap_or(Color32::WHITE);
                            
                            // cell 座標（含 leading_space）
                            let leading = main_galley.job.sections
                                .get(glyph.section_index as usize)
                                .map(|s| s.leading_space)
                                .unwrap_or(0.0);
                            let x = rect.min.x + glyph.pos.x - leading;
                            let y = rect.min.y + row.rect.min.y;
                            let w = leading + glyph.advance_width;
                            let h = row.rect.height();
                            
                            // 裁剪到 cell 邊界
                            let cell_rect = egui::Rect::from_min_size(
                                egui::pos2(x, y),
                                egui::vec2(w, h),
                            );
                            let clipped = painter.with_clip_rect(cell_rect);
                            
                            // 2x 字型大小字形剛好 14px 寬，填滿 cell
                            clipped.text(
                                egui::pos2(x + w * 0.5, y + h * 0.5),
                                egui::Align2::CENTER_CENTER,
                                ch,
                                box_font.clone(),
                                fg_color,
                            );
                        }
                    }
                
                ui.spacing_mut().item_spacing.y = 0.0;

                        if has_dual_color {
                            let overlay_galley = ui.fonts(|f| f.layout_job(overlay_job));
                            
                            for row in &overlay_galley.rows {
                                for glyph in &row.glyphs {
                                    if let Some(&(left_color, right_color)) = section_color_map.get(&(glyph.section_index as usize)) {
                                        let char_font = section_font_map.get(&(glyph.section_index as usize)).unwrap_or(&font_id);
                                        let char_pos = rect.min + glyph.pos.to_vec2();
                                        let char_w = glyph.advance_width;
                                        let char_rect = egui::Rect::from_min_max(
                                            egui::pos2(char_pos.x, rect.min.y + row.rect.min.y),
                                            egui::pos2(char_pos.x + char_w, rect.min.y + row.rect.max.y)
                                        );
 
                                        // 繪製左半部
                                        let left_clip = egui::Rect::from_min_max(
                                            char_rect.min,
                                            egui::pos2(char_rect.center().x, char_rect.max.y)
                                        );
                                        ui.painter().with_clip_rect(left_clip).text(
                                            char_rect.min,
                                            egui::Align2::LEFT_TOP,
                                            glyph.chr.to_string(),
                                            char_font.clone(),
                                            left_color,
                                        );

                                        // 繪製右半部
                                        let right_clip = egui::Rect::from_min_max(
                                            egui::pos2(char_rect.center().x, char_rect.min.y),
                                            char_rect.max
                                        );
                                        ui.painter().with_clip_rect(right_clip).text(
                                            char_rect.min,
                                            egui::Align2::LEFT_TOP,
                                            glyph.chr.to_string(),
                                            font_id.clone(),
                                            right_color,
                                        );
                                    }
                                }
                            }
                        }

                        // 有閃爍文字時在下一個切換點重繪
                        if has_blink {
                            let next_toggle = if blink_visible {
                                BLINK_PERIOD * BLINK_DUTY - blink_phase
                            } else {
                                BLINK_PERIOD - blink_phase
                            };
                            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(next_toggle));
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


    /// 繪製輸入區
    fn render_input_area(ui: &mut egui::Ui, session: &mut crate::session::Session, any_popup_open: bool, font_size: f32) {
        ui.horizontal(|ui| {
            // 先攔截 Tab 鍵，避免 egui 預設的焦點切換行為
            // 必須在 widget 渲染之前消耗，否則 egui 會先處理焦點切換
            let tab_pressed = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
            
            let response = ui.add(
                TextEdit::singleline(&mut session.input)
                    .desired_width(ui.available_width())
                    .font(FontId::monospace(font_size))
                    .hint_text("輸入指令...")
                    .lock_focus(true), // 防止 Tab 鍵切換焦點
            );

            // 如果當前沒有焦點在任何 widget 上，且沒有 popup 開啟，才自動聚焦到輸入框
            // 這樣可以避免搶走 Notes 或其他輸入框的焦點
            if !any_popup_open && !response.has_focus() && ui.ctx().memory(|m| m.focused().is_none()) {
                response.request_focus();
            }

            // 按 Enter 發送
            // 檢查 IME 是否正在組字：用當前 frame 有無實際內容的 Commit/Preedit 事件判斷
            // （macOS 上 state.ime_enabled 在中文輸入法啟用時會一直為 true，不適合用來判斷）
            // 排除空白和換行的 Commit（macOS 按 Enter 不組字時會送出這些）
            let ime_composing = ui.input(|i| {
                i.events.iter().any(|e| match e {
                    egui::Event::Ime(egui::ImeEvent::Commit(s)) => {
                        !s.is_empty() && s != "\n" && s != "\r"
                    }
                    egui::Event::Ime(egui::ImeEvent::Preedit(s)) => !s.is_empty(),
                    _ => false,
                })
            });
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && response.has_focus() && !ime_composing {
                // 發送訊息 (即使是空字串也發送，以便在 MUD 中執行重複動作或保持連線)
                let raw_input = session.input.clone();
                let cmds: Vec<&str> = raw_input.split(';').map(|s| s.trim()).collect();
                
                // 如果是空字串，也當作一個空指令發送
                let cmds = if cmds.is_empty() { vec![""] } else { cmds };

                // 記錄歷史 (原始輸入)
                if !raw_input.is_empty() {
                    session.input_history.push_back(raw_input.clone());
                    if session.input_history.len() > 1000 {
                        session.input_history.pop_front();
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
                // Tab 補齊 (使用之前攔截的結果)
                if tab_pressed {
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

        // 判斷是否在循環補齊中（input 等於上次補齊結果）
        let is_cycling = session.last_completed_input.as_ref() == Some(&session.input);

        if !is_cycling {
            // 使用者手動修改了輸入 → 重設前綴為當前輸入
            session.tab_completion_prefix = Some(session.input.clone());
            session.tab_completion_index = 0;
            session.tab_completed = false;
        }
        // 循環中：保留原始 prefix，不重設 index（index 在上次 Tab 已 +1）
        
        let Some(original_prefix) = session.tab_completion_prefix.clone() else {
            return;
        };
        
        let (prefix_to_match, base_input) = if let Some(last_space_idx) = original_prefix.rfind(' ') {
            let (base, last) = original_prefix.split_at(last_space_idx + 1);
            (last.to_string(), Some(base.to_string()))
        } else {
            (original_prefix.clone(), None)
        };

        // 支援 "2.ne" -> "2.necklace" 和 "all.sc" -> "all.scroll" 的前綴補齊
        let (search_key, dot_prefix) = if let Some((idx_str, suffix)) = prefix_to_match.split_once('.') {
            if !idx_str.is_empty() && (idx_str.chars().all(|c| c.is_ascii_digit()) || idx_str.eq_ignore_ascii_case("all")) {
                 (suffix.to_string(), Some(format!("{}.", idx_str)))
            } else {
                 (prefix_to_match.clone(), None)
            }
        } else {
             (prefix_to_match.clone(), None)
        };

        if search_key.is_empty() && dot_prefix.is_none() {
            // 如果只有空白前綴且無數字索引，避免列出所有單字
            return;
        }

        let mut matches: Vec<String> = Vec::new();
        
        // 1. 補齊歷史指令
        for history in &session.input_history {
            if history.starts_with(&original_prefix) && !matches.contains(history) {
                matches.push(history.clone());
            }
        }
        
        // 2. 補齊畫面單字
        let clean_prefix = search_key.to_lowercase();
        let mut word_matches: Vec<_> = session.screen_words.iter()
            .filter(|(w, _)| w.to_lowercase().starts_with(&clean_prefix))
            .collect();
            
        word_matches.sort_by(|(a_word, a_meta), (b_word, b_meta)| {
            // 1. 最近出現的優先
            // 2. 同時間：MobId > RoomDescription > ScreenText
            // 3. 同時間同類別：較短的優先
            b_meta.last_seen.cmp(&a_meta.last_seen)
                .then_with(|| a_meta.source.priority().cmp(&b_meta.source.priority()))
                .then_with(|| a_word.len().cmp(&b_word.len()))
        });
        
        for (word, _) in word_matches {
            let mut full_match = String::new();
            if let Some(ref b) = base_input {
                full_match.push_str(b);
            }
            if let Some(ref d) = dot_prefix {
                full_match.push_str(d);
            }
            full_match.push_str(word);
            
            if !matches.contains(&full_match) {
                matches.push(full_match);
            }
        }

        // 3. 指令字典補齊（僅當輸入是命令的首個單字時）
        if base_input.is_none() && dot_prefix.is_none() {
            for cmd in session.command_dict.matches(&search_key) {
                if !matches.contains(&cmd) {
                    matches.push(cmd);
                }
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

    /// 將 egui 按鍵事件轉換為 key combo 字串格式
    fn egui_key_to_combo(key: egui::Key, modifiers: &egui::Modifiers) -> String {
        let mut parts = Vec::new();
        if modifiers.ctrl || modifiers.mac_cmd {
            parts.push("ctrl".to_string());
        }
        if modifiers.alt {
            parts.push("alt".to_string());
        }
        if modifiers.shift {
            parts.push("shift".to_string());
        }

        let key_name = match key {
            egui::Key::F1 => "f1", egui::Key::F2 => "f2", egui::Key::F3 => "f3",
            egui::Key::F4 => "f4", egui::Key::F5 => "f5", egui::Key::F6 => "f6",
            egui::Key::F7 => "f7", egui::Key::F8 => "f8", egui::Key::F9 => "f9",
            egui::Key::F10 => "f10", egui::Key::F11 => "f11", egui::Key::F12 => "f12",
            egui::Key::Num0 => "0", egui::Key::Num1 => "1", egui::Key::Num2 => "2",
            egui::Key::Num3 => "3", egui::Key::Num4 => "4", egui::Key::Num5 => "5",
            egui::Key::Num6 => "6", egui::Key::Num7 => "7", egui::Key::Num8 => "8",
            egui::Key::Num9 => "9",
            egui::Key::A => "a", egui::Key::B => "b", egui::Key::C => "c",
            egui::Key::D => "d", egui::Key::E => "e", egui::Key::F => "f",
            egui::Key::G => "g", egui::Key::H => "h", egui::Key::I => "i",
            egui::Key::J => "j", egui::Key::K => "k", egui::Key::L => "l",
            egui::Key::M => "m", egui::Key::N => "n", egui::Key::O => "o",
            egui::Key::P => "p", egui::Key::Q => "q", egui::Key::R => "r",
            egui::Key::S => "s", egui::Key::T => "t", egui::Key::U => "u",
            egui::Key::V => "v", egui::Key::W => "w", egui::Key::X => "x",
            egui::Key::Y => "y", egui::Key::Z => "z",
            egui::Key::Space => "space",
            egui::Key::Escape => "escape",
            egui::Key::Tab => "tab",
            _ => return String::new(),
        };
        parts.push(key_name.to_string());
        parts.join("+")
    }

    /// 處理快捷鍵
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context, pending_action: &mut Option<PendingAction>) {
        // Collect key press events for Lua key binding check
        let key_events: Vec<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
            i.events.iter().filter_map(|e| {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = e {
                    Some((*key, *modifiers))
                } else {
                    None
                }
            }).collect()
        });

        // Check custom Lua key bindings (takes priority over built-in shortcuts)
        let mut consumed_combos = std::collections::HashSet::new();
        if let Some(session) = self.session_manager.active_session_mut() {
            if !session.key_bindings.is_empty() {
                for (key, modifiers) in &key_events {
                    let combo = Self::egui_key_to_combo(*key, modifiers);
                    if !combo.is_empty() {
                        if let Some(code) = session.key_bindings.get(&combo).cloned() {
                            match session.script_engine.execute_inline(&code, "", &[], false) {
                                Ok(ctx_result) => session.apply_script_context(ctx_result),
                                Err(e) => tracing::error!("Key binding '{}' error: {}", combo, e),
                            }
                            consumed_combos.insert(combo);
                        }
                    }
                }
            }
        }

        // Built-in shortcuts (skip keys consumed by Lua bindings)
        ctx.input(|i| {
            // F2-F4 功能鍵
            if !consumed_combos.contains("f2") && !consumed_combos.contains("f3") && !consumed_combos.contains("f4") {
                if i.key_pressed(egui::Key::F2) || i.key_pressed(egui::Key::F3) || i.key_pressed(egui::Key::F4) {
                    self.show_settings_window = true;
                }
            }

            // Ctrl+L 清除畫面
            if !consumed_combos.contains("ctrl+l") && i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                *pending_action = Some(PendingAction::ClearActiveWindow);
            }

            // Escape 關閉所有彈出視窗
            if !consumed_combos.contains("escape") && i.key_pressed(egui::Key::Escape) {
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
                    let combo = Self::egui_key_to_combo(*key, &i.modifiers);
                    if !consumed_combos.contains(&combo) && i.key_pressed(*key) {
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
                if !consumed_combos.contains("ctrl+t") && i.key_pressed(egui::Key::T) {
                    self.show_profile_window = true;
                }

                // Cmd+= 放大字型
                if i.key_pressed(egui::Key::Equals) {
                    self.global_config.ui.font_size = (self.global_config.ui.font_size + 1.0).min(24.0);
                    self.save_config();
                }
                // Cmd+- 縮小字型
                if i.key_pressed(egui::Key::Minus) {
                    self.global_config.ui.font_size = (self.global_config.ui.font_size - 1.0).max(10.0);
                    self.save_config();
                }
                // Cmd+0 重置字型大小
                if !consumed_combos.contains("ctrl+0") && i.key_pressed(egui::Key::Num0) {
                    self.global_config.ui.font_size = 14.0;
                    self.save_config();
                }
            }
        });
    }

}

impl eframe::App for MudApp {
    /// 攔截原始輸入：九宮格 Numpad 行走
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // 收集 numpad 按鍵對應的方向指令
        let mut commands_to_send: Vec<String> = Vec::new();
        let mut numpad_indices: Vec<usize> = Vec::new();
        let mut texts_to_remove: Vec<String> = Vec::new();

        for (i, event) in raw_input.events.iter().enumerate() {
            if let egui::Event::Key { key, pressed: true, is_numpad: true, modifiers, .. } = event {
                // 有修飾鍵時不攔截（保留 Cmd+數字 分頁切換等功能）
                if modifiers.alt || modifiers.command || modifiers.mac_cmd || modifiers.ctrl {
                    continue;
                }
                // Key → numpad index
                let index = match key {
                    egui::Key::Num0 => Some(0u8),
                    egui::Key::Num1 => Some(1),
                    egui::Key::Num2 => Some(2),
                    egui::Key::Num3 => Some(3),
                    egui::Key::Num4 => Some(4),
                    egui::Key::Num5 => Some(5),
                    egui::Key::Num6 => Some(6),
                    egui::Key::Num7 => Some(7),
                    egui::Key::Num8 => Some(8),
                    egui::Key::Num9 => Some(9),
                    egui::Key::Period => Some(10),
                    _ => None,
                };
                if let Some(idx) = index {
                    if let Some(cmd) = self.global_config.numpad.command_for_index(idx) {
                        commands_to_send.push(cmd.to_string());
                        numpad_indices.push(i);
                        // 對應的 Text 事件也要移除
                        let text_char = if idx <= 9 {
                            std::char::from_digit(idx as u32, 10).map(|c| c.to_string())
                        } else {
                            Some(".".to_string())
                        };
                        if let Some(tc) = text_char {
                            texts_to_remove.push(tc);
                        }
                    }
                }
            }
        }

        if !commands_to_send.is_empty() {
            // 從後往前移除已攔截的 Key 事件
            for &i in numpad_indices.iter().rev() {
                raw_input.events.remove(i);
            }
            // 移除對應的 Text 事件
            raw_input.events.retain(|event| {
                if let egui::Event::Text(text) = event {
                    !texts_to_remove.contains(text)
                } else {
                    true
                }
            });

            // 直接發送，不等下一幀
            if let Some(session) = self.session_manager.active_session_mut() {
                for cmd in commands_to_send {
                    Self::send_direction_for_session(session, &cmd);
                }
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // === 1. 背景邏輯處理 ===
        
        // 檢查自動重連
        self.check_reconnect(ctx);

        // 處理待連線的 Profile
        if let Some(profile_name) = self.pending_connect_profile.take() {
            self.connect_to_profile(&profile_name, ctx.clone());
        }

        // 處理所有 Session 的計時器（即使非活躍分頁也要執行）
        for session in self.session_manager.sessions_mut() {
            session.check_timers();
        }
        
        // 計算最近的計時器到期時間以喚醒 UI（包含 SM timeouts）
        let mut next_wake: Option<std::time::Duration> = None;
        let now = Instant::now();
        for session in self.session_manager.sessions_mut() {
            for timer in &session.active_timers {
                let remaining = timer.expires_at.saturating_duration_since(now);
                match next_wake {
                    None => next_wake = Some(remaining),
                    Some(d) if remaining < d => next_wake = Some(remaining),
                    _ => {}
                }
            }
            // Also consider state machine timeouts
            if !session.state_machines.machines.is_empty() {
                if let Some(sm_timeout) = session.state_machines.next_timeout() {
                    let remaining = sm_timeout.saturating_duration_since(now);
                    match next_wake {
                        None => next_wake = Some(remaining),
                        Some(d) if remaining < d => next_wake = Some(remaining),
                        _ => {}
                    }
                }
            }
        }
        if let Some(duration) = next_wake {
            ctx.request_repaint_after(duration + std::time::Duration::from_millis(10));
        }

        // 繪製其他視窗
        // Note: profiles and settings are already handled above in the floating section
        
        let mut needs_save = false;
        
        // 準備編輯器所需的 Context (依據 Scope 決定傳入 Session 或 Global Config)
        let (session_opt, global_opt) = match self.settings_scope {
            SettingsScope::Profile => (self.session_manager.active_session_mut(), None),
            SettingsScope::Global => (None, Some(&mut self.global_config)),
        };
        
        if self.show_alias_window {
            Self::render_alias_edit(
                ctx,
                session_opt,
                global_opt,
                &mut self.editing_alias_name,
                &mut self.alias_edit_name,
                &mut self.alias_edit_pattern,
                &mut self.alias_edit_replacement,
                &mut self.alias_edit_category,
                &mut self.alias_edit_is_script,
                &mut self.show_alias_window,
                &mut needs_save,
            );
        }
        
        // 重新獲取 mutable references 因為上面的 session_opt 借用結束了? 
        // Rust borrow checker 可能會抱怨 session_opt 被用兩次。
        // 但 session_opt 是 Option<&mut Session>, 不能 Copy。
        // 我們需要再次 match 或是 clone (不行).
        // 簡單解法：再次獲取。
        
        let (session_opt_trigger, global_opt_trigger) = match self.settings_scope {
            SettingsScope::Profile => (self.session_manager.active_session_mut(), None),
            SettingsScope::Global => (None, Some(&mut self.global_config)),
        };

        if self.show_trigger_window {
            Self::render_trigger_edit(
                ctx,
                session_opt_trigger,
                global_opt_trigger,
                &mut self.editing_trigger_name,
                &mut self.trigger_edit_name,
                &mut self.trigger_edit_pattern,
                &mut self.trigger_edit_action,
                &mut self.trigger_edit_category,
                &mut self.trigger_edit_is_script,
                &mut self.trigger_edit_pattern_type,
                &mut self.show_trigger_window,
                &mut needs_save,
            );
        }
        if self.show_path_window {
            Self::render_path_edit(
                ctx,
                self.session_manager.active_session_mut(),
                &mut self.editing_path_name,
                &mut self.path_edit_name,
                &mut self.path_edit_value,
                &mut self.path_edit_category,
                &mut self.show_path_window,
                &mut needs_save,
            );
        }
        if needs_save {
            self.save_config();
        }

        // 處理網路訊息（有新訊息時觸發重繪）
        if self.process_messages() {
            ctx.request_repaint();
        }

        // 主題僅在變更時才設定（避免每幀建構 Visuals + Arc clone）
        {
            let want_dark = self.global_config.ui.dark_mode;
            if ctx.style().visuals.dark_mode != want_dark {
                ctx.set_visuals(if want_dark { egui::Visuals::dark() } else { egui::Visuals::light() });
            }
        }

        // 使用局部變數記錄
        let active_id = self.session_manager.active_id();
        let any_popup_open = self.show_settings_window || self.show_alias_window || self.show_trigger_window || self.show_profile_window;
        let active_window_id = self.active_window_id.clone();

        // 記錄待執行的延遲動作（避免在閉包中借用 self）
        let mut pending_action = None;

        // === 2. UI 渲染 ===

        // === 頂部：三行工具列 ===
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            // --- Row 1: 伺服器資訊 + 狀態 + 連線按鈕 ---
            ui.horizontal(|ui| {
                if let Some(session) = self.session_manager.active_session() {
                    use crate::session::ConnectionStatus as SessionStatus;

                    ui.label(format!("伺服器: {} : {}", session.host, session.port));
                    ui.separator();

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
                            let lag_text = session.lag_monitor.display_text();
                            let color = match session.lag_monitor.lag_color() {
                                Some(ms) if ms > 2000 => Color32::RED,
                                Some(ms) if ms > 500 => Color32::YELLOW,
                                Some(_) => Color32::LIGHT_GREEN,
                                None => Color32::GRAY,
                            };
                            ui.separator();
                            ui.label(RichText::new(format!("⏱ {}", lag_text)).color(color));
                        }
                        SessionStatus::Reconnecting => {
                            ui.spinner();
                            ui.label(RichText::new("重連中...").color(Color32::YELLOW));
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
                    ui.label(RichText::new("MUD Client").strong());
                }
            });

            ui.separator();

            // --- Row 2: F1/F2/F3 按鈕 + 九宮格切換 ---
            ui.horizontal(|ui| {
                if ui.button("F1 說明").on_hover_text("設定中心").clicked() {
                    pending_action = Some(PendingAction::ToggleSettings);
                }
                if ui.button("F2 別名").on_hover_text("別名管理").clicked() {
                    self.show_alias_window = !self.show_alias_window;
                }
                if ui.button("F3 觸發").on_hover_text("觸發管理").clicked() {
                    self.show_trigger_window = !self.show_trigger_window;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let numpad_label = if self.global_config.numpad.enabled {
                        RichText::new("🎮 九宮格 ON").color(Color32::GREEN)
                    } else {
                        RichText::new("🎮 九宮格 OFF").color(Color32::GRAY)
                    };
                    if ui.button(numpad_label).on_hover_text("切換九宮格行走模式").clicked() {
                        self.global_config.numpad.enabled = !self.global_config.numpad.enabled;
                        let _ = self.global_config.save();
                    }
                });
            });

            ui.separator();

            // --- Row 3: Session 分頁標籤 + ➕ ---
            ui.horizontal(|ui| {
                if self.session_manager.len() > 0 {
                    let mut close_id = None;
                    for i in 0..self.session_manager.len() {
                        let is_active = i == self.session_manager.active_index();
                        if let Some(s) = self.session_manager.sessions().get(i) {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    if ui.selectable_label(is_active, s.tab_title()).clicked() {
                                        pending_action = Some(PendingAction::SwitchTab(i));
                                    }
                                    if ui.add(egui::Button::new("x").small().frame(false)).clicked() {
                                        close_id = Some(s.id);
                                    }
                                });
                            });
                        }
                    }
                    if let Some(id) = close_id {
                        pending_action = Some(PendingAction::CloseSession(id));
                    }
                }

                if ui.button("➕").on_hover_text("新增連線").clicked() {
                    pending_action = Some(PendingAction::ToggleProfile);
                }
            });
        });

        // === 右側：工具面板 ===
        self.render_side_panel(ctx, active_window_id.clone(), active_id, &mut pending_action);

        // === 底部：輸入區 ===
        if let Some(id) = active_id {
            egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
                if let Some(session) = self.session_manager.get_mut(id) {
                    ui.add_space(5.0);
                    let suppress_autofocus = any_popup_open;
                    Self::render_input_area(ui, session, suppress_autofocus, self.global_config.ui.font_size);
                    ui.add_space(5.0);
                }
            });

            // === 中央：訊息區 ===
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(session) = self.session_manager.get_mut(id) {
                    Self::render_message_area(ui, session, &active_window_id, self.global_config.ui.font_size, self.global_config.ui.enable_blink);
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
                PendingAction::CloseSession(id) => {
                    // 先發送斷線指令給網路執行緒
                    if let Some(session) = self.session_manager.get_mut(id) {
                        if let Some(tx) = session.command_tx.take() {
                            let _ = tx.blocking_send(crate::session::Command::Disconnect);
                        }
                    }
                    self.session_manager.close_session(id);
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

        // 有活躍連線或終端運行時以低頻率輪詢新訊息
        let has_active_connection = self.session_manager.sessions().iter().any(|s| {
            matches!(s.status, crate::session::ConnectionStatus::Connected(_) | crate::session::ConnectionStatus::Connecting)
        });
        let terminal_active = self.side_panel_tab == SidePanelTab::Terminal && self.terminal_manager.is_running();
        if has_active_connection || terminal_active {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
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
    CloseSession(crate::session::SessionId),
}


/// 簡易時間戳記（避免引入大型時間庫）
pub(crate) fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// 清理 pattern 字串，移除可能的 Debug 格式（如 Contains("...")）
pub(crate) fn clean_pattern_string(pattern: &str) -> String {
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
