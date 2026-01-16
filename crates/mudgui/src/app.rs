//! MUD Client 主要 UI 邏輯

// No unused sync imports needed anymore

use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, TextEdit};
use eframe::egui::text::LayoutJob;
use mudcore::{
    Alias, AliasManager, Logger, ScriptEngine, TelnetClient, Trigger, TriggerAction,
    TriggerManager, TriggerPattern, WindowManager, WindowMessage,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::ansi::parse_ansi;

/// UI 視圖模式
#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Main,     // 顯示訊息視窗
    Settings, // 顯示設定介面
}

/// 連線狀態
#[derive(Debug, Clone, PartialEq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected(String), // 包含伺服器資訊
}

/// MUD 客戶端 GUI 應用程式
pub struct MudApp {
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

    /// 當前選中的視窗 ID
    active_window_id: String,

    /// 當前 UI 視圖模式
    view_mode: ViewMode,
}

/// 發送給網路執行緒的命令
#[derive(Debug)]
enum Command {
    Connect(String, u16),
    Send(String),
    Disconnect,
}

impl MudApp {
    /// 創建新的 MUD 客戶端應用程式
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 設定字型
        Self::configure_fonts(&cc.egui_ctx);

        // 創建 Tokio 運行時
        let runtime = Runtime::new().expect("無法創建 Tokio 運行時");

        let mut alias_manager = AliasManager::new();
        // 預設別名範例
        alias_manager.add(Alias::new("kk", "kk", "kill kobold"));
        alias_manager.add(Alias::new("h", "h", "help"));

        let mut trigger_manager = TriggerManager::new();
        // 預設觸發器範例
        trigger_manager.add(
            Trigger::new("系統公告", TriggerPattern::Contains("系統公告".to_string()))
                .add_action(TriggerAction::Highlight { r: 255, g: 255, b: 0 }),
        );

        Self {
            runtime,
            window_manager: WindowManager::new(),
            alias_manager,
            trigger_manager,
            script_engine: ScriptEngine::new(),
            logger: Logger::new(),
            input: String::new(),
            status: ConnectionStatus::Disconnected,
            command_tx: None,
            message_rx: None,
            host: "void7777.ddns.net".to_string(),
            port: "7777".to_string(),
            auto_scroll: true,
            input_history: Vec::new(),
            history_index: None,
            active_window_id: "main".to_string(),
            view_mode: ViewMode::Main,
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

            // 將 CJK 字型添加到所有字型家族的 fallback
            for family in [
                egui::FontFamily::Monospace,
                egui::FontFamily::Proportional,
            ] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("cjk".to_owned());
            }

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

        // 嘗試載入常見的中文字型
        let font_names = [
            FamilyName::Title("PingFang TC".to_string()),
            FamilyName::Title("PingFang SC".to_string()),
            FamilyName::Title("Heiti TC".to_string()),
            FamilyName::Title("Heiti SC".to_string()),
            FamilyName::Title("Microsoft JhengHei".to_string()),
            FamilyName::Title("Noto Sans CJK TC".to_string()),
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
        self.input.clear();

        // 只有非空訊息才儲存到歷史
        if !text.is_empty() {
            self.input_history.push(text.clone());
        }
        self.history_index = None;

        // 別名處理
        let expanded = self.alias_manager.process(&text);

        if let Some(tx) = &self.command_tx {
            // 如果別名展開後包含多個命令（以分號分隔），則分開發送
            for cmd in expanded.split(';') {
                let cmd = cmd.trim();
                if !cmd.is_empty() {
                    let _ = tx.blocking_send(Command::Send(cmd.to_string()));
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
                    for action in &trigger.actions {
                        match action {
                            TriggerAction::SendCommand(cmd) => {
                                let mut expanded = cmd.clone();
                                for (i, cap) in m.captures.iter().enumerate() {
                                    expanded = expanded.replace(&format!("${}", i + 1), cap);
                                }
                                if let Some(tx) = &self.command_tx {
                                    let _ = tx.blocking_send(Command::Send(expanded));
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
                                            let _ = tx.blocking_send(Command::Send(cmd));
                                        }
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
                } else if msg.contains("連線已關閉") || msg.contains("已斷開連線") {
                    self.status = ConnectionStatus::Disconnected;
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
                            job.append(
                                &span.text,
                                0.0,
                                egui::TextFormat {
                                    font_id: font_id.clone(),
                                    color,
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
            self.view_mode = ViewMode::Settings;
        }
        
        if ui.button("返回遊戲").clicked() {
            self.view_mode = ViewMode::Main;
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.checkbox(&mut self.auto_scroll, "自動捲動");
        });
    }

    /// 繪製設定與管理介面
    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("管理中心");
        ui.separator();

        ui.collapsing("別名 (Alias)", |ui| {
            for alias in self.alias_manager.list() {
                ui.label(format!("{} -> {}", alias.pattern, alias.replacement));
            }
            if self.alias_manager.list().is_empty() {
                ui.label("尚無別名");
            }
        });

        ui.collapsing("觸發器 (Trigger)", |ui| {
            for trigger in self.trigger_manager.list() {
                ui.label(format!("{} [{:?}]", trigger.name, trigger.pattern));
            }
            if self.trigger_manager.list().is_empty() {
                ui.label("尚無觸發器");
            }
        });
        
        ui.collapsing("日誌 (Logger)", |ui| {
            if self.logger.is_recording() {
                ui.label(format!("狀態: 正在記錄中 ({:?})", self.logger.path().unwrap_or(std::path::Path::new(""))));
            } else {
                ui.label("狀態: 未啟動");
            }
        });

        ui.add_space(20.0);
            
        if ui.button("關閉並返回").clicked() {
            self.view_mode = ViewMode::Main;
        }
    }

    /// 繪製輸入區
    fn render_input_area(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let response = ui.add(
                TextEdit::singleline(&mut self.input)
                    .desired_width(ui.available_width() - 80.0)
                    .font(FontId::monospace(14.0))
                    .hint_text("輸入指令..."),
            );

            // 始終保持輸入框 focus
            if !response.has_focus() {
                response.request_focus();
            }

            // 按 Enter 發送（當輸入框有 focus 時）
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.send_message();
            }

            // 歷史導航（上/下箭頭）
            if response.has_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.navigate_history(-1);
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.navigate_history(1);
                }
            }

            if ui.button("發送").clicked() {
                self.send_message();
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
}

impl eframe::App for MudApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 處理網路訊息
        self.process_messages();

        // 設定暗黑模式
        ctx.set_visuals(egui::Visuals::dark());

        // 側邊欄
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(120.0)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // 頂部面板：連線設定
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            self.render_connection_panel(ui, ctx);
            ui.add_space(5.0);
        });

        // 底部面板：輸入區
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            self.render_input_area(ui);
            ui.add_space(5.0);
        });

        // 中央面板：訊息顯示區或設定區
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view_mode {
                ViewMode::Main => self.render_message_area(ui),
                ViewMode::Settings => self.render_settings(ui),
            }
        });

        // 持續刷新 UI 以獲取新訊息
        ctx.request_repaint();
    }
}
