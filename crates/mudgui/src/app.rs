//! MUD Client 主要 UI 邏輯

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, TextEdit};
use eframe::egui::text::LayoutJob;
use mudcore::{
    Alias, AliasManager, Logger, ScriptEngine, TelnetClient, Trigger, TriggerAction,
    TriggerManager, TriggerPattern, WindowManager, WindowMessage,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::ansi::parse_ansi;
use crate::config::{AppConfig, AliasConfig, TriggerConfig};


/// 連線狀態
#[derive(Debug, Clone, PartialEq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected(String), // 包含伺服器資訊
    Reconnecting,      // 正在等待重連
}

/// MUD 客戶端 GUI 應用程式
pub struct MudApp {
#[allow(dead_code)]
    /// Tokio 運行時
    runtime: Runtime,

    /// 輸入框內容
    input: String,

    /// 連線狀態
    status: ConnectionStatus,

    /// 發送訊息到網路執行緒的 channel
    command_tx: Option<mpsc::Sender<Command>>,

    /// 從網路執行緒接收訊息的 channel
    message_rx: Option<mpsc::Receiver<String>>,

    /// 連線設定
    host: String,
    port: String,

    /// 是否自動滾動到底部
    auto_scroll: bool,

    /// 視窗管理器（包含主視窗與子視窗）
    window_manager: WindowManager,

    /// 別名管理器
    alias_manager: AliasManager,

    /// 觸發器管理器
    trigger_manager: TriggerManager,

    /// 腳本引擎
    script_engine: ScriptEngine,

    /// 日誌記錄器
    logger: Logger,

    /// 輸入歷史
    input_history: Vec<String>,
    history_index: Option<usize>,
    
    /// Tab 補齊狀態
    tab_completion_prefix: Option<String>,
    tab_completion_index: usize,

    /// 當前選中的視窗 ID
    active_window_id: String,

    /// 連線開始時間
    connected_at: Option<Instant>,

    // === 別名編輯狀態 ===
    /// 是否顯示別名編輯視窗
    show_alias_window: bool,
    /// 正在編輯的別名名稱（None = 新增）
    editing_alias_name: Option<String>,
    /// 別名編輯框：觸發詞
    alias_edit_pattern: String,
    /// 別名編輯框：替換內容
    alias_edit_replacement: String,

    // === 觸發器編輯狀態 ===
    /// 是否顯示觸發器編輯視窗
    show_trigger_window: bool,
    editing_trigger_name: Option<String>,
    trigger_edit_name: String,
    trigger_edit_pattern: String,
    trigger_edit_action: String,
    /// 是否使用 Lua 腳本模式
    trigger_edit_is_script: bool,

    // === 設定視窗狀態 ===
    /// 是否顯示設定中心視窗
    show_settings_window: bool,

    // === 自動重連 ===
    /// 是否啟用自動重連
    auto_reconnect: bool,
    /// 重連等待時間點
    reconnect_delay_until: Option<Instant>,
    /// egui Context 的參照（用於自動重連時觸發連線）
    ctx: Option<egui::Context>,
}

/// 發送給網路執行緒的命令
#[derive(Debug)]
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

        // 載入設定
        let config = AppConfig::load();
        
        // 從設定初始化別名管理器
        let mut alias_manager = AliasManager::new();
        if config.aliases.is_empty() {
            // 無設定時使用預設範例
            alias_manager.add(Alias::new("kk", "kk", "kill kobold"));
            alias_manager.add(Alias::new("h", "h", "help"));
        } else {
            for alias_cfg in &config.aliases {
                let mut alias = Alias::new(&alias_cfg.name, &alias_cfg.pattern, &alias_cfg.replacement);
                alias.enabled = alias_cfg.enabled;
                alias_manager.add(alias);
            }
        }

        // 從設定初始化觸發器管理器
        let mut trigger_manager = TriggerManager::new();
        if config.triggers.is_empty() {
            // 無設定時使用預設範例
            trigger_manager.add(
                Trigger::new("系統公告", TriggerPattern::Contains("系統公告".to_string()))
                    .add_action(TriggerAction::Highlight { r: 255, g: 255, b: 0 }),
            );
        } else {
            for trigger_cfg in &config.triggers {
                // 清理可能的 Debug 格式（舊配置檔相容）
                let clean_pattern = clean_pattern_string(&trigger_cfg.pattern);
                
                let mut trigger = Trigger::new(
                    &trigger_cfg.name,
                    TriggerPattern::Contains(clean_pattern),
                );
                if !trigger_cfg.action.is_empty() {
                    trigger = trigger.add_action(TriggerAction::SendCommand(trigger_cfg.action.clone()));
                }
                trigger.enabled = trigger_cfg.enabled;
                trigger_manager.add(trigger);
            }
        }

        // 連線設定
        let host = if config.connection.host.is_empty() {
            "void7777.ddns.net".to_string()
        } else {
            config.connection.host.clone()
        };
        let port = if config.connection.port.is_empty() {
            "7777".to_string()
        } else {
            config.connection.port.clone()
        };

        Self {
            runtime,
            window_manager: WindowManager::new(),
            alias_manager,
            trigger_manager,
            script_engine: ScriptEngine::new(),
            logger: {
                let mut logger = Logger::new();
                // 自動啟動日誌記錄
                let log_path = format!("mud_log_{}.txt", chrono_lite_timestamp());
                let _ = logger.start(&log_path);
                tracing::info!("自動啟動日誌記錄：{}", log_path);
                logger
            },
            input: String::new(),
            status: ConnectionStatus::Disconnected,
            command_tx: None,
            message_rx: None,
            host,
            port,
            auto_scroll: true,
            input_history: Vec::new(),
            history_index: None,
            tab_completion_prefix: None,
            tab_completion_index: 0,
            active_window_id: "main".to_string(),
            connected_at: None,
            // 別名編輯狀態
            show_alias_window: false,
            editing_alias_name: None,
            alias_edit_pattern: String::new(),
            alias_edit_replacement: String::new(),
            // 觸發器編輯狀態
            show_trigger_window: false,
            editing_trigger_name: None,
            trigger_edit_name: String::new(),
            trigger_edit_pattern: String::new(),
            trigger_edit_action: String::new(),
            trigger_edit_is_script: false,
            // 設定視窗狀態
            show_settings_window: false,
            // 自動重連
            auto_reconnect: true,
            reconnect_delay_until: None,
            ctx: None,
        }
    }

    /// 儲存設定到檔案
    fn save_config(&self) {
        let config = AppConfig {
            connection: crate::config::ConnectionConfig {
                host: self.host.clone(),
                port: self.port.clone(),
            },
            aliases: self.alias_manager.list().iter().map(|a| AliasConfig {
                name: a.name.clone(),
                pattern: a.pattern.clone(),
                replacement: a.replacement.clone(),
                enabled: a.enabled,
            }).collect(),
            triggers: self.trigger_manager.list().iter().map(|t| {
                // 提取 pattern 字串
                let pattern_str = match &t.pattern {
                    TriggerPattern::Contains(s) => s.clone(),
                    TriggerPattern::StartsWith(s) => s.clone(),
                    TriggerPattern::EndsWith(s) => s.clone(),
                    TriggerPattern::Regex(s) => s.clone(),
                };
                // 提取第一個 SendCommand 或 ExecuteScript 動作
                let action_str = t.actions.iter().find_map(|a| {
                    match a {
                        TriggerAction::SendCommand(cmd) => Some(cmd.clone()),
                        TriggerAction::ExecuteScript(code) => Some(code.clone()),
                        _ => None,
                    }
                }).unwrap_or_default();
                
                TriggerConfig {
                    name: t.name.clone(),
                    pattern: pattern_str,
                    action: action_str,
                    enabled: t.enabled,
                }
            }).collect(),
        };
        let _ = config.save();
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

    /// 啟動網路連線
    fn start_connection(&mut self, ctx: egui::Context) {
        let host = self.host.clone();
        let port: u16 = self.port.parse().unwrap_or(7777);

        // 創建 channels
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(32);
        let (msg_tx, msg_rx) = mpsc::channel::<String>(1024);

        self.command_tx = Some(cmd_tx.clone());
        self.message_rx = Some(msg_rx);
        self.status = ConnectionStatus::Connecting;

        // 啟動網路執行緒
        self.runtime.spawn(async move {
            let mut client = TelnetClient::default();

            // 處理命令
            loop {
                tokio::select! {
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            Command::Connect(h, p) => {
                                match client.connect(&h, p).await {
                                    Ok(_) => {
                                        let _ = msg_tx.send(format!(">>> 已連線到 {}:{}\n", h, p)).await;

                                        // 開始讀取迴圈
                                        loop {
                                            tokio::select! {
                                                result = client.read() => {
                                                    match result {
                                                        Ok(text) if !text.is_empty() => {
                                                            // 只通過 channel 發送，不在這裡 push
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
                                                        Command::Send(text) => {
                                                            if let Err(e) = client.send(&text).await {
                                                                let _ = msg_tx.send(format!(">>> 發送失敗: {}\n", e)).await;
                                                            }
                                                        }
                                                        Command::Disconnect => {
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
                            Command::Disconnect => break,
                            _ => {}
                        }
                    }
                    else => break,
                }
            }
        });

        // 發送連線命令
        if let Some(tx) = &self.command_tx {
            let _ = tx.blocking_send(Command::Connect(host, port));
        }
    }

    /// 發送訊息（允許空訊息以發送純 Enter）
    fn send_message(&mut self) {
        let text = self.input.clone();
        // zMUD 風格：發送後不清除內容，改在 UI 端全選
        // self.input.clear();

        // 只有非空訊息才儲存到歷史
        if !text.is_empty() {
            self.input_history.push(text.clone());
        }
        self.history_index = None;

        // 別名處理
        let expanded = self.alias_manager.process(&text);

        if let Some(tx) = &self.command_tx {
            // 如果輸入為空，直接發送空字串（MUD 需要空 Enter）
            if expanded.is_empty() {
                let _ = tx.blocking_send(Command::Send(String::new()));
                // 空 Enter 回顯
                self.window_manager.route_message(
                    "main",
                    WindowMessage {
                        content: "\n".to_string(),
                        preserve_ansi: false,
                    },
                );
            } else {
                // 回顯原始輸入（緊隨提示字元）
                self.window_manager.route_message(
                    "main",
                    WindowMessage {
                        content: format!("{}\n", text),
                        preserve_ansi: false,
                    },
                );

                // 如果別名展開後包含多個命令（以分號分隔），則分開發送
                for cmd in expanded.split(';') {
                    let cmd = cmd.trim();
                    if !cmd.is_empty() {
                        let _ = tx.blocking_send(Command::Send(cmd.to_string()));
                    }
                }
            }
        }
    }

    /// 斷開連線
    fn disconnect(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.blocking_send(Command::Disconnect);
        }
        self.command_tx = None;
        self.message_rx = None;
        self.status = ConnectionStatus::Disconnected;
        // 手動斷線時停止自動重連
        self.reconnect_delay_until = None;
    }

    /// 檢查並執行自動重連
    fn check_reconnect(&mut self, ctx: &egui::Context) {
        if let ConnectionStatus::Reconnecting = self.status {
            if let Some(until) = self.reconnect_delay_until {
                if Instant::now() >= until {
                    // 時間到，執行重連
                    self.reconnect_delay_until = None;
                    self.start_connection(ctx.clone());
                } else {
                    // 持續刷新 UI 以更新倒數顯示
                    ctx.request_repaint();
                }
            }
        }
    }

    /// 處理接收到的訊息
    fn process_messages(&mut self) {
        if let Some(rx) = &mut self.message_rx {
            while let Ok(msg) = rx.try_recv() {
                // 觸發器處理
                if self.trigger_manager.should_gag(&msg) {
                    continue; // 訊息被抑制
                }

                // 處理所有匹配的觸發器動作
                let matches = self.trigger_manager.process(&msg);
                
                // 預設路由目標（主視窗）
                let mut targets = vec!["main".to_string()];
                
                for (trigger, m) in matches {
                    tracing::info!("[Trigger] 匹配觸發器: {}, 動作數: {}", trigger.name, trigger.actions.len());
                    for action in &trigger.actions {
                        tracing::info!("[Trigger] 動作類型: {:?}", std::mem::discriminant(action));
                        match action {
                            TriggerAction::SendCommand(cmd) => {
                                let mut expanded = cmd.clone();
                                for (i, cap) in m.captures.iter().enumerate() {
                                    expanded = expanded.replace(&format!("${}", i + 1), cap);
                                }
                                // 支援用 ; 分隔多個命令
                                let commands: Vec<&str> = expanded.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                                tracing::info!("[Trigger] 執行 SendCommand: {} (拆分為 {} 個命令)", expanded, commands.len());
                                if let Some(tx) = &self.command_tx {
                                    for single_cmd in commands {
                                        let _ = tx.blocking_send(Command::Send(single_cmd.to_string()));
                                        
                                        // 自動指令回顯（單獨成行）
                                        self.window_manager.route_message(
                                            "main",
                                            WindowMessage {
                                                content: format!("\n[AUTO] {}\n", single_cmd),
                                                preserve_ansi: false,
                                            },
                                        );
                                    }
                                }
                            }
                            TriggerAction::RouteToWindow(win_id) => {
                                targets.push(win_id.clone());
                            }
                            TriggerAction::ExecuteScript(code) => {
                                if let Ok(context) = self.script_engine.execute_inline(code, &msg, &m.captures) {
                                    // 執行腳本產生的命令
                                    if let Some(tx) = &self.command_tx {
                                        for cmd in context.commands {
                                            let _ = tx.blocking_send(Command::Send(cmd.clone()));
                                            
                                            // 腳本指令回顯（單獨成行）
                                            self.window_manager.route_message(
                                                "main",
                                                WindowMessage {
                                                    content: format!("\n[SCRIPT] {}\n", cmd),
                                                    preserve_ansi: false,
                                                },
                                            );
                                        }
                                    }
                                    
                                    // 處理 echos - 本地顯示
                                    for echo_text in context.echos {
                                        self.window_manager.route_message(
                                            "main",
                                            WindowMessage {
                                                content: format!(">>> {}\n", echo_text),
                                                preserve_ansi: false,
                                            },
                                        );
                                    }
                                    
                                    // 處理 window_outputs - 子視窗輸出
                                    for (win_id, text) in context.window_outputs {
                                        self.window_manager.route_message(
                                            &win_id,
                                            WindowMessage {
                                                content: format!("{}\n", text),
                                                preserve_ansi: true,
                                            },
                                        );
                                    }
                                    
                                    // 處理 log_messages - 寫入日誌
                                    for log_msg in context.log_messages {
                                        let _ = self.logger.log(&format!("[Script] {}", log_msg));
                                    }
                                    
                                    // 處理 timers - 暫時僅記錄（完整實現需要 pending_timers 欄位）
                                    for (delay_ms, timer_code) in context.timers {
                                        tracing::info!("[Timer] 將在 {}ms 後執行: {}", delay_ms, timer_code);
                                        // TODO: 加入 pending_timers 欄位並在 update() 中處理
                                    }
                                    
                                    // 處理腳本中的 Gag
                                    if context.gag {
                                        return; // 此訊息被腳本抑制，不再繼續處理
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // 路由到視窗
                for target_id in targets {
                    self.window_manager.route_message(
                        &target_id,
                        WindowMessage {
                            content: msg.clone(),
                            preserve_ansi: true,
                        },
                    );
                }

                // 日誌記錄
                let _ = self.logger.log(&msg);

                // 更新連線狀態 (從主視窗訊息判斷)
                if msg.contains("已連線到") {
                    let info = msg.replace(">>> 已連線到 ", "").replace("\n", "");
                    self.status = ConnectionStatus::Connected(info);
                    self.connected_at = Some(Instant::now());
                } else if msg.contains("連線已關閉") || msg.contains("已斷開連線") {
                    self.connected_at = None;
                    // 自動重連邏輯
                    if self.auto_reconnect {
                        use std::time::Duration;
                        self.reconnect_delay_until = Some(Instant::now() + Duration::from_secs(3));
                        self.status = ConnectionStatus::Reconnecting;
                    } else {
                        self.status = ConnectionStatus::Disconnected;
                    }
                }
            }
        }
    }

    /// 繪製連線設定面板
    fn render_connection_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label("主機:");
            ui.add(TextEdit::singleline(&mut self.host).desired_width(200.0));
            ui.label("連接埠:");
            ui.add(TextEdit::singleline(&mut self.port).desired_width(60.0));

            match &self.status {
                ConnectionStatus::Disconnected => {
                    if ui.button("連線").clicked() {
                        self.start_connection(ctx.clone());
                    }
                }
                ConnectionStatus::Connecting => {
                    ui.spinner();
                    ui.label("連線中...");
                }
                ConnectionStatus::Connected(info) => {
                    ui.label(RichText::new(format!("● 已連線 ({})", info)).color(Color32::GREEN));
                    if ui.button("斷線").clicked() {
                        self.disconnect();
                    }
                }
                ConnectionStatus::Reconnecting => {
                    ui.spinner();
                    ui.label("重連中...");
                    if ui.button("取消").clicked() {
                        self.reconnect_delay_until = None;
                        self.status = ConnectionStatus::Disconnected;
                    }
                }
            }
        });
    }

    /// 繪製訊息顯示區（支援 ANSI 顏色）
    fn render_message_area(&self, ui: &mut egui::Ui) {
        let available_height = ui.available_height() - 40.0; // 保留輸入區空間

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(available_height)
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                let font_id = FontId::monospace(14.0);

                if let Some(window) = self.window_manager.get(&self.active_window_id) {
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
    }

    /// 繪製側邊欄
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("視窗");
        ui.separator();

        for window in self.window_manager.windows() {
            let is_active = window.id == self.active_window_id;
            if ui.selectable_label(is_active, &window.title).clicked() {
                self.active_window_id = window.id.clone();
            }
        }

        ui.add_space(20.0);
        ui.heading("工具");
        ui.separator();
        
        if ui.button("中心管理").clicked() {
            self.show_settings_window = true;
        }


        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.checkbox(&mut self.auto_scroll, "自動捲動");
        });
    }

    /// 繪製設定與管理介面
    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("管理中心");
        ui.separator();

        // === 別名管理 ===
        ui.collapsing("別名 (Alias)", |ui| {
            // 新增按鈕
            if ui.button("➕ 新增別名").clicked() {
                self.editing_alias_name = Some(String::new());
                self.alias_edit_pattern = String::new();
                self.alias_edit_replacement = String::new();
                self.show_alias_window = true;
            }

            ui.add_space(10.0);

            // 別名列表
            let aliases: Vec<_> = self.alias_manager.list().iter().cloned().collect();
            let mut to_delete: Option<String> = None;
            let mut to_edit: Option<(String, String, String)> = None;
            let alias_empty = aliases.is_empty();

            for alias in &aliases {
                ui.horizontal(|ui| {
                    // 啟用/停用開關
                    let enabled_text = if alias.enabled { "✓" } else { "○" };
                    ui.label(enabled_text);

                    // 別名資訊
                    ui.label(format!("{} → {}", alias.pattern, alias.replacement));

                    // 編輯按鈕
                    if ui.small_button("✏️").clicked() {
                        to_edit = Some((alias.name.clone(), alias.pattern.clone(), alias.replacement.clone()));
                    }

                    // 刪除按鈕
                    if ui.small_button("🗑️").clicked() {
                        to_delete = Some(alias.name.clone());
                    }
                });
            }

            if alias_empty {
                ui.label("尚無別名，點擊「新增別名」開始");
            }

            // 處理刪除
            if let Some(name) = to_delete {
                self.alias_manager.remove(&name);
                self.save_config();
            }

            // 處理編輯
            if let Some((name, pattern, replacement)) = to_edit {
                self.editing_alias_name = Some(name);
                self.alias_edit_pattern = pattern;
                self.alias_edit_replacement = replacement;
                self.show_alias_window = true;
            }
        });

        // === 觸發器管理 ===
        ui.collapsing("觸發器 (Trigger)", |ui| {
            if ui.button("➕ 新增觸發器").clicked() {
                self.editing_trigger_name = Some(String::new());
                self.trigger_edit_name = String::new();
                self.trigger_edit_pattern = String::new();
                self.trigger_edit_action = String::new();
                self.show_trigger_window = true;
            }

            ui.add_space(10.0);

            let triggers: Vec<_> = self.trigger_manager.list().iter().cloned().collect();
            let mut to_delete: Option<String> = None;
            let mut to_edit: Option<(String, String, String, bool)> = None;
            let trigger_empty = triggers.is_empty();

            for trigger in &triggers {
                ui.horizontal(|ui| {
                    let enabled_text = if trigger.enabled { "✓" } else { "○" };
                    ui.label(enabled_text);
                    
                    // 人性化顯示觸發器模式
                    let pattern_text = match &trigger.pattern {
                        TriggerPattern::Contains(s) => format!("包含: {}", s),
                        TriggerPattern::StartsWith(s) => format!("開頭: {}", s),
                        TriggerPattern::EndsWith(s) => format!("結尾: {}", s),
                        TriggerPattern::Regex(s) => format!("正則: {}", s),
                    };
                    ui.label(format!("{} [{}]", trigger.name, pattern_text));

                    if ui.small_button("✏️").clicked() {
                        // 提取純文字模式
                        let clean_pattern = match &trigger.pattern {
                            TriggerPattern::Contains(s) => s.clone(),
                            TriggerPattern::StartsWith(s) => s.clone(),
                            TriggerPattern::EndsWith(s) => s.clone(),
                            TriggerPattern::Regex(s) => s.clone(),
                        };
                        
                        // 提取第一個 SendCommand 或 ExecuteScript 動作
                        let (action_str, is_script) = trigger.actions.iter().find_map(|a| {
                            match a {
                                TriggerAction::SendCommand(cmd) => Some((cmd.clone(), false)),
                                TriggerAction::ExecuteScript(code) => Some((code.clone(), true)),
                                _ => None,
                            }
                        }).unwrap_or_default();
                        
                        to_edit = Some((trigger.name.clone(), clean_pattern, action_str, is_script));
                    }
                    if ui.small_button("🗑️").clicked() {
                        to_delete = Some(trigger.name.clone());
                    }
                });
            }

            if trigger_empty {
                ui.label("尚無觸發器");
            }

            if let Some(name) = to_delete {
                self.trigger_manager.remove(&name);
                self.save_config();
            }

            if let Some((name, pattern, action, is_script)) = to_edit {
                self.editing_trigger_name = Some(name.clone());
                self.trigger_edit_name = name;
                self.trigger_edit_pattern = pattern;
                self.trigger_edit_action = action;
                self.trigger_edit_is_script = is_script;
                self.show_trigger_window = true;
            }
        });

        // === 日誌控制 ===
        ui.collapsing("日誌 (Logger)", |ui| {
            if self.logger.is_recording() {
                ui.label(format!("狀態: 正在記錄中 ({:?})", self.logger.path().unwrap_or(std::path::Path::new(""))));
                if ui.button("停止記錄").clicked() {
                    let _ = self.logger.stop();
                }
            } else {
                ui.label("狀態: 未啟動");
                if ui.button("開始記錄").clicked() {
                    let path = format!("mud_log_{}.txt", chrono_lite_timestamp());
                    let _ = self.logger.start(&path);
                }
            }
        });
    }

    /// 繪製別名編輯介面
    fn render_alias_edit(&mut self, ui: &mut egui::Ui) {
        let is_new = self.editing_alias_name.as_ref().map_or(true, |n| n.is_empty());
        ui.heading(if is_new { "新增別名" } else { "編輯別名" });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("觸發詞:");
            ui.text_edit_singleline(&mut self.alias_edit_pattern);
        });

        ui.horizontal(|ui| {
            ui.label("替換為:");
            ui.text_edit_singleline(&mut self.alias_edit_replacement);
        });

        ui.add_space(10.0);
        ui.label("提示: 使用 $1, $2 等作為參數佔位符");
        ui.label("範例: 觸發詞「go $1」替換為「walk $1;look」");

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("💾 儲存").clicked() {
                if !self.alias_edit_pattern.is_empty() {
                    // 如果是編輯模式，先刪除舊的
                    if let Some(ref old_name) = self.editing_alias_name {
                        if !old_name.is_empty() {
                            self.alias_manager.remove(old_name);
                        }
                    }
                    // 新增別名
                    self.alias_manager.add(Alias::new(
                        &self.alias_edit_pattern,
                        &self.alias_edit_pattern,
                        &self.alias_edit_replacement,
                    ));
                    self.save_config();
                    self.show_settings_window = true;
                }
            }

            if ui.button("取消").clicked() {
                self.show_settings_window = true;
            }
        });
    }

    /// 繪製觸發器編輯介面
    fn render_trigger_edit(&mut self, ui: &mut egui::Ui) {
        let is_new = self.editing_trigger_name.as_ref().map_or(true, |n| n.is_empty());
        ui.heading(if is_new { "新增觸發器" } else { "編輯觸發器" });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("名稱:");
            ui.text_edit_singleline(&mut self.trigger_edit_name);
        });

        ui.horizontal(|ui| {
            ui.label("匹配文字:");
            ui.text_edit_singleline(&mut self.trigger_edit_pattern);
        });

        ui.horizontal(|ui| {
            ui.label("執行命令:");
            ui.text_edit_singleline(&mut self.trigger_edit_action);
        });

        ui.add_space(10.0);
        ui.label("當收到包含「匹配文字」的訊息時，自動發送「執行命令」");

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("💾 儲存").clicked() {
                if !self.trigger_edit_name.is_empty() && !self.trigger_edit_pattern.is_empty() {
                    // 如果是編輯模式，先刪除舊的
                    if let Some(ref old_name) = self.editing_trigger_name {
                        if !old_name.is_empty() {
                            self.trigger_manager.remove(old_name);
                        }
                    }
                    // 新增觸發器
                    let mut trigger = Trigger::new(
                        &self.trigger_edit_name,
                        TriggerPattern::Contains(self.trigger_edit_pattern.clone()),
                    );
                    if !self.trigger_edit_action.is_empty() {
                        if self.trigger_edit_is_script {
                            trigger = trigger.add_action(TriggerAction::ExecuteScript(self.trigger_edit_action.clone()));
                        } else {
                            trigger = trigger.add_action(TriggerAction::SendCommand(self.trigger_edit_action.clone()));
                        }
                    }
                    self.trigger_manager.add(trigger);
                    self.save_config();
                    self.show_settings_window = true;
                }
            }

            if ui.button("取消").clicked() {
                self.show_settings_window = true;
            }
        });
    }

    /// 繪製輸入區
    fn render_input_area(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let response = ui.add(
                TextEdit::singleline(&mut self.input)
                    .desired_width(ui.available_width())
                    .font(FontId::monospace(14.0))
                    .hint_text("輸入指令..."),
            );

            // 只在沒有彈出視窗時才強制 focus 輸入框
            let any_popup_open = self.show_settings_window || self.show_alias_window || self.show_trigger_window;
            if !any_popup_open && !response.has_focus() {
                response.request_focus();
            }

            // 按 Enter 發送（當輸入框有 focus 時，或沒有彈出視窗開啟時）
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter_pressed && (response.has_focus() || !any_popup_open) {
                self.send_message();
                
                // zMUD 風格：發送後全選文字且保持 focus
                response.request_focus();
                
                // 手動設置全選
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.input.chars().count()),
                    )));
                    egui::TextEdit::store_state(ui.ctx(), response.id, state);
                }
            }

            // 歷史導航（上/下箭頭）
            if response.has_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.navigate_history(-1);
                    self.tab_completion_prefix = None; // 清除 Tab 補齊狀態
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.navigate_history(1);
                    self.tab_completion_prefix = None; // 清除 Tab 補齊狀態
                }
                // Tab 補齊歷史指令
                if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                    self.tab_complete();
                }
            }
        });
    }

    /// 導航輸入歷史
    fn navigate_history(&mut self, direction: i32) {
        if self.input_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            Some(idx) => {
                let new = idx as i32 + direction;
                if new < 0 {
                    0
                } else if new >= self.input_history.len() as i32 {
                    self.history_index = None;
                    self.input.clear();
                    return;
                } else {
                    new as usize
                }
            }
            None if direction < 0 => self.input_history.len() - 1,
            None => return,
        };

        self.history_index = Some(new_index);
        self.input = self.input_history[new_index].clone();
    }

    /// Tab 補齊歷史指令
    fn tab_complete(&mut self) {
        if self.input.is_empty() {
            self.tab_completion_prefix = None;
            return;
        }
        
        // 檢查使用者是否手動修改了輸入（不再匹配已存的前綴）
        if let Some(ref prefix) = self.tab_completion_prefix {
            // 如果當前輸入不是以前綴開頭，或者輸入就是前綴本身（使用者重新輸入）
            // 則視為新的補齊開始
            if !self.input.starts_with(prefix) || &self.input == prefix {
                // 使用者改變了輸入，重置狀態，以當前輸入作為新前綴
                self.tab_completion_prefix = Some(self.input.clone());
                self.tab_completion_index = 0;
            }
        } else {
            // 第一次按 Tab，記錄當前輸入作為前綴
            self.tab_completion_prefix = Some(self.input.clone());
            self.tab_completion_index = 0;
        }
        
        let prefix = self.tab_completion_prefix.clone().unwrap();
        
        // 過濾匹配前綴的歷史（從新到舊）
        let matches: Vec<_> = self.input_history.iter()
            .rev()
            .filter(|h| h.starts_with(&prefix) && *h != &prefix)
            .collect();
        
        if matches.is_empty() {
            return;
        }
        
        // 取得當前索引對應的匹配項
        let idx = self.tab_completion_index % matches.len();
        self.input = matches[idx].clone();
        
        // 下一次 Tab 時跳到下一個匹配項
        self.tab_completion_index = (self.tab_completion_index + 1) % matches.len();
    }

    /// 發送方向指令
    fn send_direction(&mut self, dir: &str) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.blocking_send(Command::Send(dir.to_string()));
        }
    }

    /// 處理快捷鍵
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // F1-F5 功能鍵
            if i.key_pressed(egui::Key::F1) {
                // TODO: 顯示說明
            }
            if i.key_pressed(egui::Key::F2) || i.key_pressed(egui::Key::F3) || i.key_pressed(egui::Key::F4) {
                self.show_settings_window = true;
            }
            if i.key_pressed(egui::Key::F5) {
                // TODO: 切換日誌
            }

            // 數字鍵盤方向（暫時禁用，避免輸入時誤觸發）
            // TODO: 改用小鍵盤專用按鍵或添加修飾鍵控制
            // if i.key_pressed(egui::Key::Num8) { self.send_direction("n"); }
            // if i.key_pressed(egui::Key::Num2) { self.send_direction("s"); }
            // if i.key_pressed(egui::Key::Num4) { self.send_direction("w"); }
            // if i.key_pressed(egui::Key::Num6) { self.send_direction("e"); }
            // if i.key_pressed(egui::Key::Num7) { self.send_direction("nw"); }
            // if i.key_pressed(egui::Key::Num9) { self.send_direction("ne"); }
            // if i.key_pressed(egui::Key::Num1) { self.send_direction("sw"); }
            // if i.key_pressed(egui::Key::Num3) { self.send_direction("se"); }
            // if i.key_pressed(egui::Key::Num5) { self.send_direction("look"); }

            // Ctrl+L 清除畫面
            if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                if let Some(window) = self.window_manager.get_mut(&self.active_window_id) {
                    window.clear();
                }
            }

            // Escape 關閉所有彈出視窗
            if i.key_pressed(egui::Key::Escape) {
                self.show_settings_window = false;
                self.show_alias_window = false;
                self.show_trigger_window = false;
            }
        });
    }
}

impl eframe::App for MudApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 儲存 context 以供自動重連使用
        self.ctx = Some(ctx.clone());

        // 檢查自動重連
        self.check_reconnect(ctx);

        // 處理網路訊息
        self.process_messages();

        // 設定暗黑模式
        ctx.set_visuals(egui::Visuals::dark());

        // === 頂部：狀態列 + 功能鍵 ===
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            // 第一行：狀態列
            ui.horizontal(|ui| {
                ui.label("伺服器:");
                ui.label(RichText::new(&self.host).strong());
                ui.label(":");
                ui.label(&self.port);
                ui.separator();

                match &self.status {
                    ConnectionStatus::Disconnected => {
                        ui.label(RichText::new("● 未連線").color(Color32::GRAY));
                    }
                    ConnectionStatus::Connecting => {
                        ui.spinner();
                        ui.label(RichText::new("連線中...").color(Color32::YELLOW));
                    }
                    ConnectionStatus::Connected(_) => {
                        ui.label(RichText::new("● 已連線").color(Color32::GREEN));
                        if let Some(start) = self.connected_at {
                            let elapsed = start.elapsed();
                            let mins = elapsed.as_secs() / 60;
                            let secs = elapsed.as_secs() % 60;
                            ui.separator();
                            ui.label(format!("時長: {:02}:{:02}", mins, secs));
                        }
                    }
                    ConnectionStatus::Reconnecting => {
                        ui.spinner();
                        if let Some(until) = self.reconnect_delay_until {
                            let remaining = until.saturating_duration_since(Instant::now());
                            ui.label(RichText::new(format!("⟳ 重連中... ({}s)", remaining.as_secs() + 1)).color(Color32::YELLOW));
                        } else {
                            ui.label(RichText::new("⟳ 重連中...").color(Color32::YELLOW));
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match &self.status {
                        ConnectionStatus::Disconnected => {
                            if ui.button("🔌 連線").clicked() {
                                self.start_connection(ctx.clone());
                            }
                        }
                        ConnectionStatus::Connected(_) => {
                            if ui.button("❌ 斷線").clicked() {
                                self.disconnect();
                            }
                        }
                        ConnectionStatus::Reconnecting => {
                            if ui.button("⏹ 取消重連").clicked() {
                                self.reconnect_delay_until = None;
                                self.status = ConnectionStatus::Disconnected;
                            }
                        }
                        _ => {}
                    }
                });
            });

            ui.separator();

            // 第二行：功能鍵
            ui.horizontal(|ui| {
                if ui.button("F1 說明").clicked() {
                    // TODO
                }
                if ui.button("F2 別名").clicked() {
                    self.show_settings_window = true;
                }
                if ui.button("F3 觸發").clicked() {
                    self.show_settings_window = true;
                }
                if ui.button("F4 腳本").clicked() {
                    self.show_settings_window = true;
                }
                if ui.button("F5 日誌").clicked() {
                    // TODO
                }

                ui.separator();
                ui.checkbox(&mut self.auto_scroll, "自動捲動");
            });
        });

        // === 右側：工具欄 ===
        egui::SidePanel::right("tools_panel")
            .resizable(true)
            .default_width(140.0)
            .min_width(100.0)
            .show(ctx, |ui| {
                ui.heading("視窗");
                ui.separator();

                for window in self.window_manager.windows() {
                    let is_active = window.id == self.active_window_id;
                    if ui.selectable_label(is_active, &window.title).clicked() {
                        self.active_window_id = window.id.clone();
                    }
                }

                ui.add_space(15.0);
                ui.heading("管理");
                ui.separator();

                if ui.button("⚙ 設定中心").clicked() {
                    self.show_settings_window = true;
                }

                ui.add_space(15.0);
                ui.heading("日誌");
                ui.separator();

                if self.logger.is_recording() {
                    ui.label(RichText::new("● 記錄中").color(Color32::RED));
                    if ui.button("停止記錄").clicked() {
                        let _ = self.logger.stop();
                    }
                } else {
                    ui.label("○ 未記錄");
                    if ui.button("開始記錄").clicked() {
                        let path = format!("mud_log_{}.txt", chrono_lite_timestamp());
                        let _ = self.logger.start(&path);
                    }
                }
            });

        // === 底部：輸入區 ===
        egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            self.render_input_area(ui);
            ui.add_space(5.0);
        });

        // === 中央：訊息區 ===
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_message_area(ui);
        });

        // === 別名編輯彈出視窗 ===
        if self.show_alias_window {
            let is_new = self.editing_alias_name.as_ref().map_or(true, |n| n.is_empty());
            let title = if is_new { "新增別名" } else { "編輯別名" };
            
            egui::Window::new(title)
                .resizable(true)
                .default_width(400.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("觸發詞:");
                        ui.text_edit_singleline(&mut self.alias_edit_pattern);
                    });
                    ui.horizontal(|ui| {
                        ui.label("替換為:");
                        ui.text_edit_singleline(&mut self.alias_edit_replacement);
                    });
                    ui.add_space(5.0);
                    ui.label("提示: 使用 $1, $2 作為參數佔位符");
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("💾 儲存").clicked() {
                            if !self.alias_edit_pattern.is_empty() {
                                if let Some(ref old_name) = self.editing_alias_name {
                                    if !old_name.is_empty() {
                                        self.alias_manager.remove(old_name);
                                    }
                                }
                                self.alias_manager.add(Alias::new(
                                    &self.alias_edit_pattern,
                                    &self.alias_edit_pattern,
                                    &self.alias_edit_replacement,
                                ));
                                self.save_config();
                                self.show_alias_window = false;
                            }
                        }
                        if ui.button("取消").clicked() {
                            self.show_alias_window = false;
                        }
                    });
                });
        }

        // === 觸發器編輯彈出視窗 ===
        if self.show_trigger_window {
            let is_new = self.editing_trigger_name.as_ref().map_or(true, |n| n.is_empty());
            let title = if is_new { "新增觸發器" } else { "編輯觸發器" };
            
            egui::Window::new(title)
                .resizable(true)
                .default_width(450.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(RichText::new("觸發器會在收到的訊息中搜尋「匹配文字」，找到時自動執行「執行命令」").small());
                    ui.add_space(10.0);
                    
                    // 名稱
                    ui.horizontal(|ui| {
                        ui.label("名稱：");
                        ui.add(TextEdit::singleline(&mut self.trigger_edit_name)
                            .hint_text("例如：自動撿取")
                            .desired_width(250.0));
                    });
                    
                    ui.add_space(5.0);
                    
                    // 匹配文字
                    ui.horizontal(|ui| {
                        ui.label("匹配文字：");
                        ui.add(TextEdit::singleline(&mut self.trigger_edit_pattern)
                            .hint_text("例如：掉落了")
                            .desired_width(250.0));
                    });
                    ui.label(RichText::new("  ↳ 當收到的訊息包含這段文字時觸發").weak().small());
                    
                    ui.add_space(5.0);
                    
                    // Lua 腳本模式勾選框
                    ui.checkbox(&mut self.trigger_edit_is_script, "使用 Lua 腳本");
                    
                    // 執行命令
                    ui.horizontal(|ui| {
                        ui.label("執行命令：");
                        if self.trigger_edit_is_script {
                            ui.add(TextEdit::multiline(&mut self.trigger_edit_action)
                                .hint_text("mud.send(\"get all\")\nmud.echo(\"OK\")")
                                .desired_width(250.0)
                                .desired_rows(3));
                        } else {
                            ui.add(TextEdit::singleline(&mut self.trigger_edit_action)
                                .hint_text("get all")
                                .desired_width(250.0));
                        }
                    });
                    if self.trigger_edit_is_script {
                        ui.label(RichText::new("  ↳ Lua 腳本模式，使用 mud.send(\"...\") 發送命令").weak().small());
                    } else {
                        ui.label(RichText::new("  ↳ 直接發送命令到 MUD").weak().small());
                    }
                    
                    ui.add_space(15.0);
                    
                    // 範例區塊
                    ui.collapsing("📖 使用範例", |ui| {
                        ui.label("• 簡單模式：輸入 get all");
                        ui.label("• Lua 模式（多指令）：");
                        ui.monospace("mud.send(\"get all\")\nmud.send(\"put all in bag\")");
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("💾 儲存").clicked() {
                            if !self.trigger_edit_name.is_empty() && !self.trigger_edit_pattern.is_empty() {
                                if let Some(ref old_name) = self.editing_trigger_name {
                                    if !old_name.is_empty() {
                                        self.trigger_manager.remove(old_name);
                                    }
                                }
                                let mut trigger = Trigger::new(
                                    &self.trigger_edit_name,
                                    TriggerPattern::Contains(self.trigger_edit_pattern.clone()),
                                );
                                if !self.trigger_edit_action.is_empty() {
                                    if self.trigger_edit_is_script {
                                        trigger = trigger.add_action(TriggerAction::ExecuteScript(self.trigger_edit_action.clone()));
                                    } else {
                                        trigger = trigger.add_action(TriggerAction::SendCommand(self.trigger_edit_action.clone()));
                                    }
                                }
                                self.trigger_manager.add(trigger);
                                self.save_config();
                                self.show_trigger_window = false;
                            }
                        }
                        if ui.button("取消").clicked() {
                            self.show_trigger_window = false;
                        }
                    });
                });
        }

        // === 設定中心彈出視窗 ===
        if self.show_settings_window {
            egui::Window::new("設定中心")
                .resizable(true)
                .default_width(500.0)
                .default_height(400.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    self.render_settings(ui);
                    
                    ui.add_space(10.0);
                    if ui.button("關閉").clicked() {
                        self.show_settings_window = false;
                    }
                });
        }

        // 處理快捷鍵
        self.handle_keyboard_shortcuts(ctx);

        // 持續刷新
        ctx.request_repaint();
    }
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
