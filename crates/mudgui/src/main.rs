//! MUD Client GUI 應用程式

mod ansi;
mod app;
mod config;

use app::MudApp;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> eframe::Result<()> {
    // 初始化日誌
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // 設定 eframe 選項
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("MUD Client"),
        ..Default::default()
    };

    // 啟動 GUI
    eframe::run_native(
        "MUD Client",
        options,
        Box::new(|cc| Ok(Box::new(MudApp::new(cc)))),
    )
}
