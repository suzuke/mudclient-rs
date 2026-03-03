//! MUD Client 主要 UI 邏輯

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, TextEdit};
use eframe::egui::text::LayoutJob;
use egui_extras::{Column, TableBuilder};
use mudcore::{
    Alias, TelnetClient, Trigger, TriggerAction,
    TriggerPattern, Path,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::api::{self, ApiStateManager};
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
    trigger_search_text: String,

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
        Self::setup_fonts(&cc.egui_ctx);

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
            alias_edit_is_script: false,
            show_trigger_window: false,
            editing_trigger_name: None,
            trigger_edit_name: String::new(),
            trigger_edit_pattern: String::new(),
            trigger_edit_action: String::new(),
            trigger_edit_category: String::new(),
            trigger_edit_is_script: false,
            
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

    /// 初始化字型設定
    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        
        // 內嵌常規與粗體字型
        let reg_bytes = include_bytes!("../assets/fonts/SarasaMonoTC-Regular.ttf");
        let bold_bytes = include_bytes!("../assets/fonts/SarasaMonoTC-Bold.ttf");
        
        // 1. 註冊常規體 (cjk)
        fonts.font_data.insert(
            "cjk".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(reg_bytes.to_vec())),
        );
        
        fonts.families.get_mut(&egui::FontFamily::Monospace)
            .map(|f| f.insert(0, "cjk".to_owned()));
        fonts.families.get_mut(&egui::FontFamily::Proportional)
            .map(|f| f.insert(0, "cjk".to_owned()));

        // 2. 註冊真正的粗體 (cjk_bold)
        fonts.font_data.insert(
            "cjk_bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bold_bytes.to_vec())),
        );
        fonts.families.insert(
            egui::FontFamily::Name("cjk_bold".into()),
            vec!["cjk_bold".to_owned()],
        );

        tracing::info!("字型載入狀態: 已內嵌 SarasaMonoTC Regular 與 Bold");
        ctx.set_fonts(fonts);
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
                                                        SessionCommand::CollectResponse { command, callback_code } => {
                                                            // === Phase 0: 先把 channel 裡所有等待的 Send 指令發出去 ===
                                                            while let Ok(pending_cmd) = cmd_rx.try_recv() {
                                                                match pending_cmd {
                                                                    SessionCommand::Send(text) => {
                                                                        if let Err(e) = client.send(&text).await {
                                                                            let _ = msg_tx.send(NetMessage::Text(format!(">>> 發送失敗: {}\n", e), Vec::new())).await;
                                                                        }
                                                                    }
                                                                    // 如果有另一個 CollectResponse 排在後面，暫時忽略（不應該發生）
                                                                    _ => {}
                                                                }
                                                            }

                                                            // === Phase 1: Drain — 讀盡管線中所有待處理的回應 ===
                                                            // 所有前序指令已發送，等待它們的回應全部到達
                                                            loop {
                                                                match tokio::time::timeout(
                                                                    std::time::Duration::from_millis(300),
                                                                    client.read_with_widths()
                                                                ).await {
                                                                    Ok(Ok((text, widths))) if !text.is_empty() => {
                                                                        let _ = msg_tx.send(NetMessage::Text(text, widths)).await;
                                                                        ctx.request_repaint();
                                                                    }
                                                                    _ => break, // Timeout 或錯誤 = 管線已清
                                                                }
                                                            }

                                                            // === Phase 2: 發送指令 ===
                                                            if let Err(e) = client.send(&command).await {
                                                                let _ = msg_tx.send(NetMessage::Text(format!(">>> CollectResponse 發送失敗: {}\n", e), Vec::new())).await;
                                                                continue;
                                                            }

                                                            // === Phase 3: Collect — 收集回應直到 prompt ===
                                                            let mut collected_lines: Vec<String> = Vec::new();
                                                            loop {
                                                                match tokio::time::timeout(
                                                                    std::time::Duration::from_secs(5),
                                                                    client.read_with_widths()
                                                                ).await {
                                                                    Ok(Ok((text, widths))) if !text.is_empty() => {
                                                                        // 轉發給 UI 顯示
                                                                        let _ = msg_tx.send(NetMessage::Text(text.clone(), widths)).await;
                                                                        ctx.request_repaint();

                                                                        // 解析行，加入收集
                                                                        for line in text.split('\n') {
                                                                            let clean = crate::ansi::strip_ansi(line);
                                                                            let trimmed = clean.replace('\r', "");
                                                                            let trimmed = trimmed.trim();
                                                                            if !trimmed.is_empty() {
                                                                                collected_lines.push(trimmed.to_string());
                                                                            }
                                                                        }

                                                                        // 判斷回應是否結束：不以 \n 結尾 + 50ms 確認
                                                                        if !text.ends_with('\n') {
                                                                            match tokio::time::timeout(
                                                                                std::time::Duration::from_millis(50),
                                                                                client.read_with_widths()
                                                                            ).await {
                                                                                Err(_) => {
                                                                                    // 確認：prompt，收集結束
                                                                                    // 移除最後一行（prompt 行）
                                                                                    collected_lines.pop();
                                                                                    break;
                                                                                }
                                                                                Ok(Ok((more, w))) if !more.is_empty() => {
                                                                                    // 有更多資料，繼續收集
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
                                                                    _ => break, // 超時或錯誤
                                                                }
                                                            }

                                                            // === Phase 4: 發送收集結果 ===
                                                            let _ = msg_tx.send(NetMessage::CollectedResponse {
                                                                lines: collected_lines,
                                                                callback_code,
                                                            }).await;
                                                            ctx.request_repaint();
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

    /// 發送訊息（針對指定 Session）
    fn send_message_for_session(&mut self, session: &mut crate::session::Session) {
        // 發送指令時自動捲到最底
        session.scroll_to_bottom_on_next_frame = true;
        
        let text = session.input.clone();

        // 只有非空訊息才儲存到歷史
        if !text.is_empty() {
            session.input_history.push(text.clone());
            // 限制歷史記錄數量，避免記憶體無上限增長
            if session.input_history.len() > 500 {
                session.input_history.drain(..session.input_history.len() - 500);
            }
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
        use crate::session::NetMessage;
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
                                    if let Ok(mut api) = session.api_state.lock() {
                                        api.connection_status = format!("connected:{}", info);
                                    }
                                } else if text.contains("連線已關閉") || text.contains("已斷開連線") {
                                    session.connected_at = None;
                                    if session.auto_reconnect {
                                        use std::time::Duration;
                                        session.reconnect_delay_until = Some(Instant::now() + Duration::from_secs(3));
                                        session.status = SessionStatus::Reconnecting;
                                        if let Ok(mut api) = session.api_state.lock() {
                                            api.connection_status = "reconnecting".to_string();
                                        }
                                    } else {
                                        session.status = SessionStatus::Disconnected;
                                        if let Ok(mut api) = session.api_state.lock() {
                                            api.connection_status = "disconnected".to_string();
                                        }
                                    }
                                }
                            }
                            NetMessage::CollectedResponse { lines, callback_code } => {
                                session.execute_collected_response(lines, callback_code);
                            }
                        }
                    }
                }
            }
        }
        // 處理 API 命令
        self.process_api_commands();
    }

    /// 處理 API 傳入的指令和 Lua 程式碼（多 Session 版）
    fn process_api_commands(&mut self) {
        // 遍歷所有 Session 的 ApiState，取出各自的 pending 佇列
        let all_states = self.api_state_mgr.all_states();
        
        for (session_key, api_state) in all_states {
            let (commands, lua_codes, eval_lua_codes) = {
                if let Ok(mut api) = api_state.lock() {
                    (api.drain_commands(), api.drain_lua(), api.drain_eval_lua())
                } else {
                    continue;
                }
            };

            // 跳過沒有待處理項目的 Session
            if commands.is_empty() && lua_codes.is_empty() && eval_lua_codes.is_empty() {
                continue;
            }

            // 找到對應的 Session 並執行
            let session = self.session_manager.sessions_mut().iter_mut()
                .find(|s| s.id.value().to_string() == session_key);
            
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
                        Ok(result) => result,
                        Err(e) => {
                            tracing::error!("API Eval Lua error (session {}): {}", session_key, e);
                            format!("Error: {}", e)
                        }
                    };
                    let _ = tx.send(result_str);
                }
            }
        }
    }

    /// 繪製訊息顯示區（支援 ANSI 顏色）
    fn render_message_area(ui: &mut egui::Ui, session: &mut crate::session::Session, active_window_id: &str, font_size: f32) {
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
                
                // 穩定測量：使用空格寬度作為 Mono 單元格寬度基準
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
                let mut pending_trailing_space: f32 = 0.0; // 用於置中對齊：記錄上一個字元的後半部間距
                // 字型寬度快取 — 避免每個字元都查字型系統
                let mut glyph_cache: std::collections::HashMap<(char, bool), f32> = std::collections::HashMap::new();

                if let Some(window) = session.window_manager.get(active_window_id) {
                    // 只渲染最近 N 條訊息以避免效能問題
                    let visible_lines = ((available_height / (font_size + 4.0)) as usize * 3).max(200);
                    let total = window.message_count();
                    let skip = total.saturating_sub(visible_lines);
                    for msg in window.messages().skip(skip) {
                        use crate::ansi::parse_ansi_with_widths;
                        let spans = parse_ansi_with_widths(&msg.content, Some(&msg.byte_widths));

                         
                        for span in spans {
                            let italics = span.blink;
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
                            // 判斷是否為真正的雙色字：fg_color_left 有值且 span 只有一個可見字元
                            // 多字元 span（如「紅龍護符」）→ 非雙色字，用 fg_color_left 為統一顏色
                            let visible_chars = span.text.chars().filter(|c| *c != '\n' && *c != '\r').count();
                            let is_real_dual_color = span.fg_color_left.is_some() && visible_chars == 1;
                            
                            // 非雙色字渲染
                            if !is_real_dual_color {
                                for (idx, ch) in span.text.chars().enumerate() {
                                    if ch == '\n' || ch == '\r' {
                                        let fmt = egui::TextFormat { font_id: current_font_id.clone(), color: render_color, background, italics, line_height: Some(font_size + 4.0), ..Default::default() };
                                        section_fg_colors.push(render_color);
                                        main_job.append(&ch.to_string(), pending_trailing_space, fmt.clone());
                                        overlay_job.append(&ch.to_string(), pending_trailing_space, egui::TextFormat { color: Color32::TRANSPARENT, background: Color32::TRANSPARENT, ..fmt });
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
                                    let char_color = if let Some(left_color) = span.fg_color_left {
                                        if !ch.is_ascii() { left_color } else { render_color }
                                    } else {
                                        render_color
                                    };
                                    let glyph_color = if ch >= '\u{2500}' && ch <= '\u{259f}' {
                                        Color32::TRANSPARENT
                                    } else {
                                        char_color
                                    };
                                    let fmt = egui::TextFormat {
                                        font_id: current_font_id.clone(),
                                        color: glyph_color,
                                        background,
                                        italics,
                                        line_height: Some(font_size + 4.0),
                                        ..Default::default()
                                    };
                                    section_fg_colors.push(char_color);
                                    main_job.append(&ch.to_string(), current_leading, fmt.clone());
                                    overlay_job.append(&ch.to_string(), current_leading, egui::TextFormat { color: Color32::TRANSPARENT, background: Color32::TRANSPARENT, ..fmt });
                                }
                                continue;
                            }

                            // 雙色字逐字元網格對齊模式
                            for (idx, ch) in span.text.chars().enumerate() {
                                if ch == '\n' || ch == '\r' {
                                    let fmt = egui::TextFormat { font_id: current_font_id.clone(), color: render_color, background, italics, line_height: Some(font_size + 4.0), ..Default::default() };
                                    section_fg_colors.push(render_color);
                                    main_job.append(&ch.to_string(), pending_trailing_space, fmt.clone());
                                    overlay_job.append(&ch.to_string(), pending_trailing_space, egui::TextFormat { color: Color32::TRANSPARENT, background: Color32::TRANSPARENT, ..fmt });
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
                                    italics,
                                    line_height: Some(font_size + 4.0),
                                    ..Default::default()
                                };

                                let section_idx = main_job.sections.len();
                                if let Some(left_color) = span.fg_color_left {
                                    has_dual_color = true;
                                    section_color_map.insert(section_idx, (left_color, render_color));
                                    section_font_map.insert(section_idx, current_font_id.clone());
                                    
                                    let mut overlay_fmt = format.clone();
                                    format.color = Color32::TRANSPARENT;
                                    overlay_fmt.color = Color32::WHITE;
                                    overlay_fmt.background = Color32::TRANSPARENT;
                                    
                                    section_fg_colors.push(render_color);
                                    main_job.append(&ch.to_string(), current_leading, format);
                                    overlay_job.append(&ch.to_string(), current_leading, overlay_fmt);
                                } else {
                                    let mut overlay_fmt = format.clone();
                                    overlay_fmt.color = Color32::TRANSPARENT;
                                    overlay_fmt.background = Color32::TRANSPARENT;
                                    
                                    section_fg_colors.push(render_color);
                                    main_job.append(&ch.to_string(), current_leading, format);
                                    overlay_job.append(&ch.to_string(), current_leading, overlay_fmt);
                                }
                            }
                        }

                        // 確保訊息之間有換行，並重置置中間距
                        if !main_job.text.is_empty() && !main_job.text.ends_with('\n') {
                            let nl_fmt = egui::TextFormat { font_id: font_id.clone(), line_height: Some(font_size + 4.0), ..Default::default() };
                            section_fg_colors.push(Color32::TRANSPARENT);
                            main_job.append("\n", pending_trailing_space, nl_fmt.clone());
                            overlay_job.append("\n", pending_trailing_space, egui::TextFormat { color: Color32::TRANSPARENT, ..nl_fmt });
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
                                ch.to_string(),
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
        global_config_opt: Option<&mut GlobalConfig>,
        editing_alias_name: &mut Option<String>,
        alias_edit_pattern: &mut String,
        alias_edit_replacement: &mut String,
        alias_edit_category: &mut String,
        alias_edit_is_script: &mut bool,
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
                        ui.text_edit_multiline(alias_edit_replacement);
                    } else {
                        ui.text_edit_singleline(alias_edit_replacement);
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
                                alias.is_script = *alias_edit_is_script;
                                if !alias_edit_category.is_empty() {
                                    alias.category = Some(alias_edit_category.clone());
                                }
                                session.alias_manager.add(alias);
                                *needs_save_flag = true;
                            } else if let Some(global) = global_config_opt {
                                // Global Config Logic
                                let name = if let Some(ref old_name) = editing_alias_name {
                                    if !old_name.is_empty() {
                                        // Remove old
                                        global.global_aliases.retain(|a| &a.name != old_name);
                                        old_name.clone()
                                    } else {
                                        alias_edit_pattern.clone()
                                    }
                                } else {
                                    alias_edit_pattern.clone()
                                };
                                
                                // Push new
                                global.global_aliases.push(crate::config::AliasConfig {
                                    name,
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
    fn render_trigger_edit(
        ctx: &egui::Context,
        session_opt: Option<&mut crate::session::Session>,
        global_config_opt: Option<&mut GlobalConfig>,
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
                                    || trigger_edit_pattern.contains("|")
                                    || trigger_edit_pattern.contains("?")
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
                            } else if let Some(global) = global_config_opt {
                                // Global Config Logic
                                let name = if let Some(ref old_name) = editing_trigger_name {
                                    if !old_name.is_empty() {
                                        global.global_triggers.retain(|t| &t.name != old_name);
                                        old_name.clone()
                                    } else {
                                        trigger_edit_name.clone()
                                    }
                                } else {
                                    trigger_edit_name.clone()
                                };
                                
                                global.global_triggers.push(crate::config::TriggerConfig {
                                    name,
                                    pattern: trigger_edit_pattern.clone(),
                                    action: trigger_edit_action.clone(),
                                    category: if trigger_edit_category.is_empty() { None } else { Some(trigger_edit_category.clone()) },
                                    is_script: *trigger_edit_is_script,
                                    enabled: true,
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
    fn render_path_edit(
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

    /// 繪製側邊欄
    fn render_side_panel(&mut self, ctx: &egui::Context, active_window_id: String, _active_id: Option<crate::session::SessionId>, pending_action: &mut Option<PendingAction>) {
        egui::SidePanel::right("tools_panel")
            .resizable(true)
            .default_width(250.0) // 增加寬度以容納攻略
            .show(ctx, |ui| {
                // 1. 標籤頁切換
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Tools, "🛠️ 工具");
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Guide, "📖 攻略");
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Notes, "📝 筆記");
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
                }
            });
    }

    /// 繪製工具分頁 (原有的側邊欄內容)
    fn render_tools_tab(&mut self, ui: &mut egui::Ui, active_window_id: &str, pending_action: &mut Option<PendingAction>) {
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
    fn render_guide_tab(&mut self, ui: &mut egui::Ui) {
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
    fn render_notes_tab(&mut self, ui: &mut egui::Ui) {
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
        
        let original_prefix = session.tab_completion_prefix.clone().unwrap();
        
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
                if i.key_pressed(egui::Key::Num0) {
                    self.global_config.ui.font_size = 14.0;
                    self.save_config();
                }
            }
        });
    }

    /// 繪製 Profile 管理視窗 (含連線與新增/編輯/刪除)
    fn render_profile_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("👤 連線管理")
            .default_width(400.0)
            .default_height(500.0)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Profile 列表");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ 新增 Profile").clicked() {
                            self.editing_profile_original_name = None;
                            self.profile_edit_name = String::new();
                            self.profile_edit_display_name = String::new();
                            self.profile_edit_host = "localhost".to_string();
                            self.profile_edit_port = "7777".to_string();
                            self.profile_edit_username = String::new();
                            self.profile_edit_password = String::new();
                            self.show_profile_edit_window = true;
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
                    egui::ScrollArea::vertical().max_height(ui.available_height()).show(ui, |ui| {
                        for (name, display_name, host, port, username) in &profiles {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(display_name).strong());
                                        let user_info = if let Some(u) = username { format!(" | User: {}", u) } else { String::new() };
                                        ui.label(format!("{}:{}{}", host, port, user_info));
                                    });
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        // 連線按鈕
                                        if ui.button("🔌 連線").clicked() {
                                            self.pending_connect_profile = Some(name.clone());
                                            self.show_profile_window = false;
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
                                                let new_name = format!("{}_copy", name);
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
    fn render_profile_edit_window(&mut self, ctx: &egui::Context) {
        let title = if self.editing_profile_original_name.is_some() { "✏️ 編輯 Profile" } else { "➕ 新增 Profile" };
        
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("profile_edit_grid_conn").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                    ui.label("識別名稱 (ID):");
                    if self.editing_profile_original_name.is_some() {
                        ui.label(RichText::new(&self.profile_edit_name).code()); // ID 不可修改
                    } else {
                        ui.text_edit_singleline(&mut self.profile_edit_name);
                    }
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
                                    self.alias_edit_pattern = String::new();
                                    self.alias_edit_replacement = String::new();
                                    self.alias_edit_category = String::new();
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
                        }
                        let mut op_action: Option<AliasOp> = None;

                        // 表格繪製
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::auto()) // Enabled
                            .column(Column::auto()) // Source Icon
                            .column(Column::auto().at_least(60.0)) // Category
                            .column(Column::initial(100.0).resizable(true)) // Name
                            .column(Column::initial(150.0).resizable(true)) // Pattern
                            .column(Column::remainder()) // Replacement
                            .column(Column::auto()) // Actions
                            .header(20.0, |mut header| {
                                header.col(|ui| { ui.strong("啟用"); });
                                header.col(|ui| { ui.strong("來源"); });
                                header.col(|ui| { ui.strong("分類"); });
                                header.col(|ui| { ui.strong("名稱"); });
                                header.col(|ui| { ui.strong("指令"); });
                                header.col(|ui| { ui.strong("內容"); });
                                header.col(|ui| { ui.strong("操作"); });
                            })
                            .body(|mut body| {
                                for (category, items) in grouped_aliases {
                                     let category_id_str = category.clone().unwrap_or_else(|| "default".to_string());
                                     let is_expanded_id = body.ui_mut().make_persistent_id(format!("alias_cat_{}", category_id_str));
                                     let is_expanded = body.ui_mut().data(|d| d.get_temp::<bool>(is_expanded_id).unwrap_or(true));

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
                                         row.col(|_| {});
                                     });

                                    if is_expanded {
                                        for (name, pattern, replacement, cat, enabled, is_script, source) in items {
                                            body.row(24.0, |mut row| {
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

                                                // 3. 分類
                                                row.col(|_ui| {
                                                    // ui.label(cat.as_deref().unwrap_or("-")); // Optional
                                                });

                                                // 4. 名稱
                                                row.col(|ui| {
                                                    ui.label(&name);
                                                });

                                                // 5. 指令 (Pattern)
                                                row.col(|ui| {
                                                    ui.label(&pattern).on_hover_text(&pattern);
                                                });

                                                // 6. 內容 (Replacement)
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
                                                    ui.label(display_text).on_hover_text(&replacement);
                                                });

                                                // 7. 操作
                                                row.col(|ui| {
                                                     ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 8.0;
                                                        if ui.button("✏️").on_hover_text("編輯").clicked() {
                                                            to_edit = Some((name.clone(), pattern.clone(), replacement.clone(), cat.clone().unwrap_or_default(), is_script));
                                                        }

                                                        if self.settings_scope == SettingsScope::Profile {
                                                            ui.menu_button(" ⋮ ", |ui| {
                                                                ui.set_min_width(120.0);
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

                                                        if ui.button("🗑️").on_hover_text("刪除").clicked() {
                                                            to_delete = Some(name.clone());
                                                        }
                                                     });
                                                });
                                            });
                                        }
                                    }
                                }
                            });
                        
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
                            self.editing_alias_name = Some(name);
                            self.alias_edit_pattern = pattern;
                            self.alias_edit_replacement = replacement;
                            self.alias_edit_category = category;
                            self.alias_edit_is_script = is_script;
                            self.show_alias_window = true;
                        }

                        // 處理範圍操作
                        if let Some(op) = op_action {
                            match op {
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

                        // 收集 Trigger 列表
                        let mut trigger_list: Vec<(String, String, String, Option<String>, bool, bool, String, TriggerSource)> = match self.settings_scope {
                            SettingsScope::Profile => {
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

                                            (t.name.clone(), pattern_text, clean_pattern, t.category.clone(), t.enabled, is_script, action_str, source)
                                        })
                                    })
                                    .collect()
                            },
                            SettingsScope::Global => {
                                self.global_config.global_triggers.iter().map(|t| {
                                    let pattern_text = format!("(Global) {}", t.pattern);
                                    (t.name.clone(), pattern_text, t.pattern.clone(), t.category.clone(), t.enabled, t.is_script, t.action.clone(), TriggerSource::Global)
                                }).collect()
                            }
                        };

                        // 搜尋過濾
                        let search = self.trigger_search_text.to_lowercase();
                        if !search.is_empty() {
                            trigger_list.retain(|(name, p_text, _, cat, _, _, _, _)| {
                                name.to_lowercase().contains(&search) || 
                                p_text.to_lowercase().contains(&search) ||
                                cat.as_deref().unwrap_or("").to_lowercase().contains(&search)
                            });
                        }
                        
                        // Grouping Logic
                        let mut grouped_triggers: std::collections::BTreeMap<Option<String>, Vec<(String, String, String, Option<String>, bool, bool, String, TriggerSource)>> = std::collections::BTreeMap::new();
                        for item in trigger_list {
                            grouped_triggers.entry(item.3.clone()).or_default().push(item);
                        }

                        let mut to_delete: Option<String> = None;
                        let mut to_edit: Option<(String, String, String, bool, String)> = None;
                        let mut to_toggle_name: Option<(String, bool)> = None;
                        let mut to_toggle_category: Option<(Option<String>, bool)> = None;
                        
                        // 操作 Action
                        enum TriggerOp {
                            MoveToGlobal(String),
                            MoveToProfile(String),
                            RevertToGlobal(String),
                            CopyToGlobal(String),
                        }
                        let mut op_action: Option<TriggerOp> = None;

                        // 表格繪製
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::auto()) // Enabled / Toggle
                            .column(Column::auto()) // Source Icon
                            .column(Column::auto().at_least(60.0)) // Category
                            .column(Column::initial(120.0).resizable(true)) // Name
                            .column(Column::remainder()) // Pattern
                            .column(Column::auto()) // Actions
                            .header(20.0, |mut header| {
                                header.col(|ui| { ui.strong("啟用"); });
                                header.col(|ui| { ui.strong("來源"); });
                                header.col(|ui| { ui.strong("分類"); });
                                header.col(|ui| { ui.strong("名稱"); });
                                header.col(|ui| { ui.strong("觸發內容"); });
                                header.col(|ui| { ui.strong("操作"); });
                            })
                            .body(|mut body| {
                                for (category, items) in grouped_triggers {
                                    let category_id_str = category.clone().unwrap_or_else(|| "default".to_string());
                                    let is_expanded_id = body.ui_mut().make_persistent_id(format!("trig_cat_{}", category_id_str));
                                    let is_expanded = body.ui_mut().data(|d| d.get_temp::<bool>(is_expanded_id).unwrap_or(true));

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
                                        row.col(|_| {}); // Pattern placeholder 
                                        row.col(|_| {}); // Action placeholder
                                    });

                                    if is_expanded {
                                        for (name, pattern_text, clean_pattern, cat, enabled, is_script, action_str, source) in items {
                                            body.row(24.0, |mut row| {
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

                                                // 3. 分類 (Empty in row, shown in header)
                                                row.col(|_ui| {
                                                    // ui.label(cat.as_deref().unwrap_or("-")); // Optional: Leave empty to reduce clutter
                                                });

                                                // 4. 名稱
                                                row.col(|ui| {
                                                    ui.label(&name);
                                                });

                                                // 5. 觸發內容
                                                row.col(|ui| {
                                                    ui.label(&pattern_text).on_hover_text(&pattern_text);
                                                });

                                                // 6. 操作
                                                row.col(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 8.0; 
                                                        
                                                        if ui.button("✏️").on_hover_text("編輯").clicked() {
                                                            to_edit = Some((name.clone(), clean_pattern.clone(), action_str.clone(), is_script, cat.clone().unwrap_or_default()));
                                                        }

                                                        if self.settings_scope == SettingsScope::Profile {
                                                            ui.menu_button(" ⋮ ", |ui| {
                                                                ui.set_min_width(120.0);
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
                                                                            to_edit = Some((name.clone(), clean_pattern.clone(), action_str.clone(), is_script, cat.clone().unwrap_or_default()));
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
                                                        
                                                        if ui.button("🗑️").on_hover_text("刪除").clicked() {
                                                            to_delete = Some(name.clone());
                                                        }
                                                    });
                                                });
                                            });
                                        }
                                    }
                                }
                            });
                        
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

                        if let Some((name, pattern, action, is_script, category)) = to_edit {
                            self.editing_trigger_name = Some(name.clone());
                            self.trigger_edit_name = name;
                            self.trigger_edit_pattern = pattern;
                            self.trigger_edit_action = action;
                            self.trigger_edit_category = category;
                            self.trigger_edit_is_script = is_script;
                            self.show_trigger_window = true;
                        }

                        // 處理範圍操作
                        if let Some(op) = op_action {
                            match op {
                                TriggerOp::MoveToGlobal(name) | TriggerOp::CopyToGlobal(name) => {
                                    if let Some(t) = session.trigger_manager.get(&name) {
                                        let (action_str, is_script) = t.actions.iter().find_map(|a| {
                                            match a {
                                                TriggerAction::SendCommand(cmd) => Some((cmd.clone(), false)),
                                                TriggerAction::ExecuteScript(code) => Some((code.clone(), true)),
                                                _ => None,
                                            }
                                        }).unwrap_or_default();
                                        
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
        
        // 計算最近的計時器到期時間以喚醒 UI
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
                session_opt, // 不能同時借用 self.session_manager 與 self.global_config (如果是 Global mode, session_opt 是 None, 安全)
                global_opt,
                &mut self.editing_alias_name,
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

                // 九宮格行走模式切換
                let numpad_label = if self.global_config.numpad.enabled {
                    RichText::new("🎮 九宮格 ON").color(Color32::GREEN)
                } else {
                    RichText::new("🎮 九宮格").color(Color32::GRAY)
                };
                if ui.button(numpad_label).on_hover_text("切換九宮格行走模式").clicked() {
                    self.global_config.numpad.enabled = !self.global_config.numpad.enabled;
                    let _ = self.global_config.save();
                }

                ui.separator();
                // 分頁列
                if self.session_manager.len() > 0 {
                    let mut close_id = None;
                    for i in 0..self.session_manager.len() {
                        let is_active = i == self.session_manager.active_index();
                        if let Some(s) = self.session_manager.sessions().get(i) {
                            // 使用 group 讓分頁標籤與關閉按鈕視覺上結合
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    if ui.selectable_label(is_active, s.tab_title()).clicked() {
                                        pending_action = Some(PendingAction::SwitchTab(i));
                                    }
                                    // 關閉按鈕 (x)
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
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕").clicked() {
                        pending_action = Some(PendingAction::ToggleProfile);
                    }
                });
            });
        });

        // === 右側：工具面板 ===
        self.render_side_panel(ctx, active_window_id.clone(), active_id, &mut pending_action);

        // === 底部：輸入區 ===
        if let Some(id) = active_id {
            egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
                if let Some(session) = self.session_manager.get_mut(id) {
                    ui.add_space(5.0);
                    Self::render_input_area(ui, session, any_popup_open, self.global_config.ui.font_size);
                    ui.add_space(5.0);
                }
            });

            // === 中央：訊息區 ===
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(session) = self.session_manager.get_mut(id) {
                    Self::render_message_area(ui, session, &active_window_id, self.global_config.ui.font_size);
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

        // 僅在有活躍連線時持續刷新（處理持續收到的伺服器訊息）
        let has_active_connection = self.session_manager.sessions().iter().any(|s| {
            matches!(s.status, crate::session::ConnectionStatus::Connected(_) | crate::session::ConnectionStatus::Connecting)
        });
        if has_active_connection {
            ctx.request_repaint();
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
