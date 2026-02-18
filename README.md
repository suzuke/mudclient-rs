# MUD Client (mudclient-rs)

[![Build Status](https://github.com/suzuke/mudclient-rs/actions/workflows/build.yml/badge.svg)](https://github.com/suzuke/mudclient-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一個使用 Rust 與 egui 開發的高效能、現代化 MUD 客戶端。專為中文 MUD 環境設計，解決了長期以來的編碼與顯示痛點。

## ✨ 特色功能

*   **🚀 極速體驗**: Rust 打造，啟動快、佔用低。
*   **🌏 完美中文支援**: 內建 Big5 編碼處理，修正 CJK 字元對齊問題 (`1:2` 寬度比例)。
*   **🔌 強大腳本引擎**: 完整整合 **Lua 5.4**，支援觸發器 (Triggers)、別名 (Aliases)、計時器 (Timers)。
*   **🎨 現代化介面**: 支援 256 色與 TrueColor，可自訂分頁、側邊欄工具與主題。
*   **🤖 自動化助手**: 內建路徑記錄 (`#path`)、自動地圖 (開發中) 與多種任務腳本。
*   **📜 完整紀錄**: 支援 HTML 格式日誌，保留顏色與格式，方便回顧。

## 📥 安裝與執行

### 預編譯版本 (推薦)
前往 [GitHub Releases](https://github.com/suzuke/mudclient-rs/releases) 下載適用於您可以平台的最新版本：
- **macOS** (Apple Silicon/Intel)
- **Windows** (x64)

### 從原始碼編譯
如果您熟悉 Rust 開發環境：

1.  **安裝 Rust**: [https://rustup.rs/](https://rustup.rs/)
2.  **複製專案**:
    ```bash
    git clone https://github.com/suzuke/mudclient-rs.git
    cd mudclient-rs
    ```
3.  **編譯**:
    ```bash
    cargo build -p mudgui --release
    ```
4.  **執行**:
    執行檔位於 `target/release/mudgui`。建議將 `scripts/` 目錄複製到執行檔同層。

## 📖 使用指南

### 常用指令
*   `#loop <次數> <指令>`: 重複執行指令。
*   `#delay <毫秒> <指令>`: 延遲執行。
*   `#path start/stop/back`: 路徑記錄與回溯。
*   `/lua <代碼>`: 執行 Lua 代碼。

### 文件索引
*   **[API 文件 (Lua Scripting)](docs/API.md)**: 詳細的腳本開發指南。
*   **[系統架構 (Architecture)](docs/Architecture.md)**: 了解內部運作原理。
*   **[開發路線圖 (Roadmap)](docs/RoadMap.md)**: 專案未來規劃。
*   **[環境變數 (Env)](docs/Env.md)**: 進階設定選項。
*   **[MCP Server 使用指南](docs/MCP_Usage.md)**: 外部控制 API 說明。

## 🤝 貢獻
歡迎提交 Pull Request 或 Issue。請參考 [CONTRIBUTING.md](CONTRIBUTING.md) (如有) 了解詳情。

## 📄 授權
MIT License
