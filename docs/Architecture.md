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
    *   **Triggers**: 基於 Regex 的訊息觸發器。
    *   **Aliases**: 指令別名與參數展開。
    *   **Timers**: 排程任務管理。
*   **Logger (日誌層)**:
    *   結構化日誌記錄。
    *   支援 ANSI 轉 HTML 的日誌輸出。

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
| **語言** | Rust (2021) | 核心開發語言 |
| **GUI** | egui, eframe | 跨平台圖形介面 |
| **非同步** | tokio | 網路 I/O 與任務調度 |
| **腳本** | mlua (Lua 5.4) | 使用者腳本擴充 |
| **編碼** | encoding_rs | Big5/GBK 支援 |
| **序列化** | serde, serde_json | 設定檔與資料儲存 |
| **正則** | regex | 觸發器匹配 |

## 📂 檔案系統

*   `.agent/`: Agent 相關配置與記憶 (Workflow, Rules, Knowledge)。
*   `assets/`: 字型、圖示等靜態資源。
*   `crates/`: Rust 原始碼 (Workspace)。
*   `docs/`: 專案文件。
*   `scripts/`: 預設 Lua 腳本庫。
*   `logs/`: 執行日誌與遊戲紀錄。
*   `target/`: 編譯產出。
