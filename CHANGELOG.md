# Changelog

本專案遵循 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式記錄變更。

## [0.8.0] - 2026-03-06
### Added
- **字型設定**: 允許使用者自訂字型家族，支援系統等寬/CJK 字型掃描與即時切換。
- **主題切換**: 深色/淺色模式切換，含 ANSI 前景色自適應確保兩種模式下文字可讀性。

### Changed
- **版本號**: Cargo.toml workspace 版本從 0.1.0 更新至 0.8.0，與 git tag 一致。
- **CI 修正**: 將殘留的 Node.js CI workflow 替換為 Rust cargo build/test。

### Fixed
- **Profile 驗證**: 新增 Profile 時名稱或連線資訊為空會顯示 Toast 錯誤訊息（原先靜默忽略）。

## [0.7.0 ~ 0.7.3] - 2026-03-05
### Added
- **Embedded Terminal**: 側邊欄內嵌終端模擬器 (`egui-term` crate)，可直接在客戶端內操作 Shell。
- **macOS App Icon**: 修正 icon.png 為真實 PNG 格式。

### Fixed
- 修正房間 hash ID（hash 演算法變更後更新）。
- 修正 `MudNav` 過期 timeout timer 導致 false Step Stuck。

## [0.6.0] - 2026-03-05
### Added
- **Rust MCP Server**: 以 Rust 原生重寫 MCP Server（rmcp 1.x, stdio），取代舊 Node.js/TypeScript 版本，提供 19 個工具。
- **HTTP API 擴充**: 新增 11 個端點（aliases/triggers/paths/windows/history/map 等），共 19 個 REST 端點。
- **`mud.ask_llm()` Lua API**: 觸發器可非同步呼叫 Anthropic LLM 做即時決策，支援自訂模型。

### Changed
- **Mapper 改進**: auto-save、碰撞處理、方向標籤。

### Fixed
- 修復 Mapper 導致的輸入延遲問題（5 項針對性優化）。

## [0.5.0 ~ 0.5.1] - 2026-03-04
### Added
- **Rust 原生 MapDatabase**: 取代 Lua MudMapper 的地圖資料庫，支援 auto-save、碰撞處理、方向標籤。
- **Mapper 視覺化**: egui canvas 渲染地圖、BFS 佈局、互動縮放與平移。
- **Toast UI**: Profile export/import 操作加入 Toast 通知回饋。

### Changed
- **Password Storage**: 從 OS keychain 改為 base64-encoded JSON 儲存，提升跨平台相容性。
- **Code Quality**: 全面性程式碼品質改進 (P0/P1/P2)。
- **Toolbar**: 恢復上方工具列為三列佈局。
- **Room Detection**: 改用 Telnet prompt boundary 偵測房間。

## [0.1.x] - 2026-02-12 ~ 2026-03-03
### Added
- **Numpad 方向鍵**: 支援數字鍵盤方向移動（含 egui patch 偵測 key location）。
- **Side Panel**: 側邊欄工具面板，包含 Script/Guide/Notes 分頁。
- **Log Folding**: 支援連續重複訊息折疊，減少畫面洗版。
- **Portable Build**: GitHub Actions 自動建置跨平台可攜式執行檔。
- **Tab 自動補齊**: 指令字典、Prompt 排除、字型大小調整。
- **Multi-Session MCP 支援**: 透過 session key 指定操作目標。
- **Lua 模組系統**: MudNav, MudCombat, MudLoot, MudMapper, MudExplorer。

### Changed
- **Trigger & Alias UI**: 觸發器與別名管理介面改進。
- **Messaging**: 優化訊息顯示邏輯，修正 CJK 字元對齊與亂碼問題。

### Fixed
- 修復 Windows 上 IME 輸入法組字期間誤送 Enter 的問題。
- 修復房間偵測誤將玩家/NPC 識別為房間的問題。
- 修復特定中文字串 (如 "七彩蓮花座") 顯示錯誤問題。

## [0.1.0] - 2026-02-12
### Added
- 專案初始化 (MVP)。
- 基礎 Telnet 連線與 Big5 支援。
- 核心 Lua 腳本引擎 integration。
- 基礎 UI 實作 (egui)。
