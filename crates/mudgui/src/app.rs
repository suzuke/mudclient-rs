//! MUD Client 主要 UI 邏輯

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, TextEdit};
use eframe::egui::text::LayoutJob;
use mudcore::{MessageBuffer, TelnetClient};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::ansi::parse_ansi;

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

    /// 訊息緩衝區（與網路執行緒共享）
    messages: Arc<Mutex<MessageBuffer>>,

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

    /// 輸入歷史
    input_history: Vec<String>,
    history_index: Option<usize>,
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

        Self {
            runtime,
            messages: Arc::new(Mutex::new(MessageBuffer::new(10000))),
            input: String::new(),
            status: ConnectionStatus::Disconnected,
            command_tx: None,
            message_rx: None,
            host: "void7777.ddns.net".to_string(),
            port: "7777".to_string(),
            auto_scroll: true,
            input_history: Vec::new(),
            history_index: None,
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

        let _messages = Arc::clone(&self.messages);

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

        if let Some(tx) = &self.command_tx {
            let _ = tx.blocking_send(Command::Send(text));
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
                if let Ok(mut buf) = self.messages.lock() {
                    // 檢查是否已經存在（避免重複）
                    buf.push(msg.clone());
                }

                // 更新連線狀態
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

                if let Ok(buf) = self.messages.lock() {
                    for msg in buf.iter() {
                        // 解析 ANSI 顏色碼
                        let spans = parse_ansi(msg);
                        
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
        // 處理接收到的訊息
        self.process_messages();

        // 設定深色主題
        ctx.set_visuals(egui::Visuals::dark());

        egui::TopBottomPanel::top("connection_panel").show(ctx, |ui| {
            self.render_connection_panel(ui, ctx);
        });

        egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
            self.render_input_area(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_message_area(ui);
        });
    }
}
