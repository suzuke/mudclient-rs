# Changelog

本專案遵循 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式記錄變更。

## [Unreleased]

### Added
- **Embedded Terminal**: 側邊欄內嵌終端模擬器 (`egui-term` crate)，可直接在客戶端內操作 Shell。
- **Rust MCP Server**: 以 Rust 原生重寫 MCP Server（rmcp 1.x, stdio），取代舊 Node.js/TypeScript 版本，提供 19 個工具。
- **HTTP API 擴充**: 新增 11 個端點（aliases/triggers/paths/windows/history/map 等），共 19 個 REST 端點。
- **`mud.ask_llm()` Lua API**: 觸發器可非同步呼叫 Anthropic LLM 做即時決策，支援自訂模型。
- **Rust 原生 MapDatabase**: 取代 Lua MudMapper 的地圖資料庫，支援 auto-save、碰撞處理、方向標籤。
- **Mapper 視覺化**: egui canvas 渲染地圖、BFS 佈局、互動縮放與平移。
- **Numpad 方向鍵**: 支援數字鍵盤方向移動（含 egui patch 偵測 key location）。
- **Toast UI**: Profile export/import 操作加入 Toast 通知回饋。
- **Side Panel**: 側邊欄工具面板，包含 Script/Guide/Notes 分頁。
- **Log Folding**: 支援連續重複訊息折疊，減少畫面洗版。
- **Portable Build**: GitHub Actions 自動建置跨平台可攜式執行檔。

### Changed
- **Password Storage**: 從 OS keychain 改為 base64-encoded JSON 儲存，提升跨平台相容性。
- **Room Detection**: 改用 Telnet prompt boundary 偵測房間，擴大 buffer/scan_limit，過濾 NPC 動作與系統訊息。
- **Trigger & Alias UI**: 觸發器與別名管理介面改進。
- **Toolbar**: 恢復上方工具列為三列佈局。
- **Code Quality**: 全面性程式碼品質改進 (P0/P1/P2)。
- **Messaging**: 優化訊息顯示邏輯，修正 CJK 字元對齊與亂碼問題。
- **Scripting**: 改進 `ikkoku_quest.lua` 與 `yotsuya.lua` 流程，提升任務自動化穩定性。
- **Logging**: 日誌檔案正確儲存於 `logs/` 目錄，並支援 HTML 格式。

### Fixed
- 修復 Mapper 導致的輸入延遲問題（5 項針對性優化）。
- 修復 Windows 上 IME 輸入法組字期間誤送 Enter 的問題。
- 修復房間偵測誤將玩家/NPC 識別為房間的問題。
- 修復 `MudNav` 在 Recovery 狀態下重複發送指令的問題。
- 修復 Session 關閉時網路連線未正確斷開的問題。
- 修復特定中文字串 (如 "七彩蓮花座") 顯示錯誤問題。

## [0.1.0] - 2026-01-01
### Added
- 專案初始化 (MVP)。
- 基礎 Telnet 連線與 Big5 支援。
- 核心 Lua 腳本引擎 integration.
- 基礎 UI 實作 (egui)。
