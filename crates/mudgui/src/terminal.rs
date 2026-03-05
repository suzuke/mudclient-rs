//! 終端模擬器管理
//!
//! 使用 egui-term (基於 alacritty_terminal) 在側邊欄提供嵌入式終端。

use std::sync::mpsc;

use eframe::egui;
use egui_term::{BackendSettings, PtyEvent, TerminalBackend, TerminalView};

/// 終端管理器，負責 PTY 後端的生命週期與渲染
pub struct TerminalManager {
    shell: String,
    backend: Option<TerminalBackend>,
    pty_event_rx: Option<mpsc::Receiver<(u64, PtyEvent)>>,
    pty_event_tx: mpsc::Sender<(u64, PtyEvent)>,
    has_focus: bool,
    /// 終端是否已退出（用來區分「尚未初始化」與「已退出」）
    exited: bool,
}

impl TerminalManager {
    pub fn new() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let (tx, rx) = mpsc::channel();
        Self {
            shell,
            backend: None,
            pty_event_rx: Some(rx),
            pty_event_tx: tx,
            has_focus: false,
            exited: false,
        }
    }

    /// 初始化 PTY 後端
    fn init_backend(&mut self, ctx: &egui::Context) {
        let settings = BackendSettings {
            shell: self.shell.clone(),
            args: vec![],
            working_directory: None,
        };

        match TerminalBackend::new(1, ctx.clone(), self.pty_event_tx.clone(), settings) {
            Ok(backend) => {
                self.backend = Some(backend);
                self.exited = false;
                tracing::info!("終端後端初始化成功 (shell: {})", self.shell);
            }
            Err(e) => {
                tracing::error!("終端後端初始化失敗: {}", e);
            }
        }
    }

    /// 處理 PTY 事件（退出等）
    fn process_pty_events(&mut self) {
        if let Some(rx) = &self.pty_event_rx {
            while let Ok((_id, event)) = rx.try_recv() {
                if let PtyEvent::Exit = event {
                    tracing::info!("終端程序已退出");
                    self.backend = None;
                    self.exited = true;
                }
            }
        }
    }

    /// 在 UI 區域渲染終端
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // 首次渲染時延遲初始化
        if self.backend.is_none() && !self.exited {
            self.init_backend(ui.ctx());
        }

        self.process_pty_events();

        if let Some(backend) = &mut self.backend {
            let terminal_view = TerminalView::new(ui, backend).set_focus(self.has_focus);
            ui.add(terminal_view);
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("終端已停止");
                if ui.button("重新啟動終端").clicked() {
                    self.exited = false;
                    self.init_backend(ui.ctx());
                }
            });
        }
    }

    /// 設定焦點狀態
    pub fn set_focus(&mut self, focus: bool) {
        self.has_focus = focus;
    }

    /// 終端是否正在運行
    pub fn is_running(&self) -> bool {
        self.backend.is_some()
    }
}
