# 系統架構 (Architecture)

## 🏗️ 概覽

`mudclient-rs` 採用模組化架構，將核心邏輯與圖形介面分離，以確保可測試性與跨平台相容性。

## 📦 模組結構 (Crates)

專案採用 Cargo Workspace 結構，包含以下主要成員：

### 1. `mudcore` (核心邏輯)
負責所有非 UI 的業務邏輯，是 MUD 客戶端的大腦。

*   **Net (網路層)**:
    *   基於 `tokio` 的非同步 TCP 連線。
    *   Telnet 協定解析 (IAC 指令處理)。
    *   Big5 / UTF-8 編碼轉換 (`encoding_rs`)。
*   **Scripting (腳本層)**:
    *   內嵌 Lua 5.4 虛擬機 (`mlua`)。
    *   提供 `mud.*` API 綁定。
    *   管理 Lua 狀態與全域變數。
*   **Automation (自動化)**:
    *   **Triggers**: 基於 Regex 的訊息觸發器，支援群組 (Groups) 批量開關。
    *   **Aliases**: 指令別名與參數展開。
    *   **Timers**: 排程任務管理。
*   **Event System (事件系統)**:
    *   **EventBus**: 事件匯流排，支援 `on/off/emit/once` 操作。
    *   **State Machine**: 有限狀態機框架，支援狀態轉換、timeout、enter/exit callback。
    *   **Key Bindings**: 快捷鍵綁定 (F1-F12, Ctrl/Alt 組合鍵)。
    *   **Message Routing**: 訊息路由到子視窗，支援 gag 隱藏。
*   **Logger (日誌層)**:
    *   結構化日誌記錄，支援重複行摺疊。
    *   支援 PlainText / Raw / HTML 三種格式。
    *   啟動時自動 gzip 壓縮超過 7 天的舊 log（`.txt` → `.txt.gz`）。

### 2. `mudgui` (圖形介面)
負責畫面渲染與使用者互動，是 MUD 客戶端的臉面。

*   **Framework**: 基於 `egui` (Immediate Mode GUI) 與 `eframe`。
*   **Components**:
    *   **Terminal**: 顯示 MUD 訊息，處理捲動、選取與複製。
    *   **Input**: 指令輸入框，支援歷史紀錄與補齊。
    *   **SidePanel**: 工具、筆記與指南顯示區域。
    *   **Tabs**: 多視窗/頻道管理。
*   **State Management**:
    *   維護 UI 狀態 (字型大小、顏色主題、視窗佈局)。
    *   接收來自 `mudcore` 的非同步事件並更新畫面。

## 🔄 資料流 (Data Flow)

```mermaid
graph TD
    User[使用者] -->|輸入指令| GUI[MudGUI / Input]
    GUI -->|Command Event| Core[MudCore]
    
    subgraph MudCore
        Parser[指令解析器]
        Lua[Lua 引擎]
        Net[網路層]
        Trigger[觸發器系統]
    end
    
    Core -->|1. 解析指令| Parser
    Parser -->|是別名?| Parser
    Parser -->|是腳本?| Lua
    Parser -->|是伺服器指令?| Net
    
    Net -->|送出 TCP| Server[MUD 伺服器]
    Server -->|回應資料| Net
    
    Net -->|2. 接收資料| Trigger
    Trigger -->|匹配?| Lua
    Trigger -->|處理後資料| GUI
    
    Lua -->|mud.send()| Net
    Lua -->|mud.print()| GUI
    
    GUI -->|渲染畫面| Monitor[螢幕]
```

## 🛠️ 技術棧

| 領域 | 技術/套件 | 用途 |
| :--- | :--- | :--- |
| **語言** | Rust (2021 Edition) | 核心開發語言 |
| **GUI** | egui 0.30, eframe | 跨平台圖形介面 (Immediate Mode) |
| **非同步** | tokio 1.43 | 網路 I/O 與任務調度 |
| **HTTP** | axum 0.8 | 內建 REST API Server |
| **腳本** | mlua (Lua 5.4, vendored) | 使用者腳本擴充 |
| **編碼** | encoding_rs | Big5/GBK 支援 |
| **序列化** | serde, serde_json | 設定檔與資料儲存 |
| **正則** | regex | 觸發器匹配 |
| **雜湊** | sha2 | 房間 ID 雜湊 |
| **MCP** | rmcp 1.x (Rust) | AI Agent 控制介面 (stdio) |
| **LLM** | reqwest + Anthropic API | Lua 腳本非同步 LLM 呼叫 |

### 3. `egui-term` (內嵌終端模擬器)
側邊欄嵌入的終端模擬器元件，讓使用者不離開客戶端即可操作 Shell。

*   **Backend**: 基於 PTY (pseudo-terminal) 的終端後端。
*   **Rendering**: 自訂 egui widget，處理終端字元渲染、游標與主題。
*   **Integration**: 嵌入 `mudgui` 側邊欄，獨立於 MUD 連線運作。

### 4. `mcp-server` (AI 控制介面)
Rust 原生的 MCP Server，透過 stdio 模式讓 AI Agent（如 Claude Code、Claude Desktop）操控 MUD 客戶端。

*   **協定**: Model Context Protocol (stdio 模式，rmcp 1.x)。
*   **後端通訊**: 透過 reqwest 呼叫 `mudgui` 內建的 Axum REST API。
*   **19 個工具**:
    *   **連線管理**: `list_sessions`, `get_status`
    *   **訊息**: `read_messages`, `clear_messages`, `get_command_history`
    *   **遊戲操作**: `send_command`, `get_room_info`
    *   **腳本**: `execute_lua`, `evaluate_lua`
    *   **地圖**: `get_map_stats`, `search_map_rooms`, `find_map_path`
    *   **自動化管理**: `list_aliases`, `list_triggers`, `list_paths`, `manage_alias`, `manage_trigger`
    *   **子視窗**: `list_windows`, `read_window`

### 4. `scripts/` (Lua 腳本庫)
遊戲自動化腳本集合，分為獨立腳本與共用模組。

*   **共用模組** (`scripts/modules/`):
    *   `MudNav` — 路徑導航與位置追蹤。
    *   `MudCombat` — 戰鬥邏輯與技能施放。
    *   `MudLoot` — 物品拾取與背包管理。
    *   `MudMapper` — 房間記錄與地圖建構。
    *   `MudExplorer` — 自動探索。
    *   `MudUtils` — 通用工具函式。
*   `QuestEngine` — 宣告式任務引擎，協調各模組執行多步驟任務。
*   **任務腳本**: `poker_quest_v2`, `ikkoku_quest_v2`, `smurf_quest_v2`（QuestEngine 版）, `itemfarm` 等。
*   **輔助腳本**: `autocast`, `benumb`, `practice`, `memcalc`, `skillplanner` 等。

## 📂 檔案系統

*   `.agent/`: AI Agent 配置 (Rules, Skills, Workflows, Hooks)。
*   `assets/`: 字型、圖示等靜態資源。
*   `crates/`: Rust 原始碼 (Workspace: `mudcore` + `mudgui` + `mcp-server` + `egui-term`)。
*   `data/`: 靜態資料檔 (地圖、設定)。
*   `docs/`: 專案文件。
*   `packaging/`: 打包腳本 (DMG 建置等)。
*   `scripts/`: Lua 腳本庫與共用模組。
*   `logs/`: 執行日誌與遊戲紀錄（舊檔自動壓縮為 `.txt.gz`，可用 `zcat`/`zgrep` 查詢）。
*   `target/`: 編譯產出。
*   `tests/`: 整合測試。

## 🔧 開發指引

```bash
# 編譯 (Debug)
cargo build -p mudgui

# 編譯 (Release)
cargo build -p mudgui --release

# 執行測試
cargo test --workspace

# MCP Server 編譯
cargo build -p mcp-server --release
```

## 📚 相關文件

| 文件 | 說明 |
| :--- | :--- |
| [API.md](API.md) | Lua 腳本 API 參考 |
| [ScriptGuide.md](ScriptGuide.md) | 腳本開發入門教學 |
| [EventSystem.md](EventSystem.md) | 事件系統、狀態機、快捷鍵教學 |
| [QuestEngine.md](QuestEngine.md) | 宣告式任務引擎教學 |
| [MCP_Usage.md](MCP_Usage.md) | MCP Server 使用指南 |
| [MCP_Agent_Playbook.md](MCP_Agent_Playbook.md) | AI Agent 操控 Playbook |
| [Navigation.md](Navigation.md) | 導航系統說明 |
| [Deployment.md](Deployment.md) | 部署與打包流程 |
| [Env.md](Env.md) | 環境變數設定 |
| [RoadMap.md](RoadMap.md) | 開發路線圖 |
