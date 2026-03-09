# MUD Client (mudclient-rs)

[![Build Status](https://github.com/suzuke/mudclient-rs/actions/workflows/build.yml/badge.svg)](https://github.com/suzuke/mudclient-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一個使用 Rust 與 egui 開發的高效能、現代化 MUD 客戶端。專為中文 MUD 環境設計，解決了長期以來的編碼與顯示痛點。

## ✨ 特色功能

*   **🚀 極速體驗**: Rust 打造，啟動快、佔用低。
*   **🌏 完美中文支援**: 內建 Big5 編碼處理，修正 CJK 字元對齊問題 (`1:2` 寬度比例)。
*   **🔌 強大腳本引擎**: 完整整合 **Lua 5.4**，支援觸發器 (Triggers)、別名 (Aliases)、計時器 (Timers)。
*   **🎨 現代化介面**: 支援 256 色與 TrueColor，可自訂分頁、側邊欄工具與主題。
*   **🤖 自動化助手**: 內建路徑記錄 (`#path`)、模組化任務腳本 (MudNav, MudCombat 等)。
*   **📜 完整紀錄**: 支援 HTML 格式日誌，保留顏色與格式，方便回顧。
*   **🧠 AI 整合**: 內建 [MCP Server](docs/MCP_Usage.md)，讓 AI Agent 透過標準協定操控 MUD；Lua 腳本可透過 `mud.ask_llm()` 即時呼叫 LLM 決策。
*   **🔗 多連線管理**: Multi-Session 支援，同時管理多個 MUD 連線。
*   **🖥️ 內嵌終端**: 側邊欄嵌入終端模擬器 ([egui-term](crates/egui-term))，不離開客戶端即可操作 Shell。
*   **🗺️ 地圖視覺化**: Rust 原生地圖資料庫，egui canvas 渲染、BFS 佈局、互動縮放導航。

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

### Lua 腳本模組
內建 `scripts/modules/` 提供可重用的遊戲邏輯模組：
*   **MudNav** — 路徑導航與位置追蹤
*   **MudCombat** — 戰鬥邏輯與技能施放
*   **MudLoot** — 物品拾取與背包管理
*   **MudMapper** — 房間記錄與地圖建構

### 文件索引
*   **[觸發器與別名指南 (Triggers & Aliases)](docs/Triggers_Aliases.md)**: 自動化操作的核心設定。
*   **[API 文件 (Lua Scripting)](docs/API.md)**: 詳細的腳本 API 參考。
*   **[腳本開發指南 (Script Guide)](docs/ScriptGuide.md)**: 從零開始寫腳本。
*   **[系統架構 (Architecture)](docs/Architecture.md)**: 了解內部運作原理。
*   **[開發路線圖 (Roadmap)](docs/RoadMap.md)**: 專案未來規劃。
*   **[環境變數 (Env)](docs/Env.md)**: 進階設定選項。
*   **[MCP Server 使用指南](docs/MCP_Usage.md)**: AI Agent 控制 API 說明。

## 🖥️ 系統需求

| 平台 | 最低版本 |
| :--- | :--- |
| macOS | 11.0 (Big Sur) 以上 |
| Windows | 10 (64-bit) 以上 |
| Rust (編譯用) | 1.75+ (2021 Edition) |

## 🛠️ 開發者快速開始

```bash
# 複製並編譯
git clone https://github.com/suzuke/mudclient-rs.git
cd mudclient-rs
cargo build -p mudgui

# 安裝 Git Hooks（自動同步 Cargo.toml 版本與 git tag）
cp githooks/sync-cargo-version .git/hooks/post-checkout
cp githooks/sync-cargo-version .git/hooks/post-merge

# 執行測試
cargo test --workspace

# 編譯 MCP Server (Rust 原生)
cargo build -p mcp-server --release
```

## 🤝 貢獻
歡迎提交 Pull Request 或 Issue。請參考 [CONTRIBUTING.md](CONTRIBUTING.md) (如有) 了解詳情。

## 📄 授權
MIT License
