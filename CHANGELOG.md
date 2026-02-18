# Changelog

本專案遵循 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式記錄變更。

## [Unreleased]

### Added
- **Side Panel**: 新增側邊欄工具面板，包含 Script/Guide/Notes 分頁。
- **Log Folding**: 支援連續重複訊息折疊，減少畫面洗版。
- **Portable Build**: 新增 GitHub Actions 流程，自動建置跨平台可攜式執行檔。

### Changed
- **Messaging**: 優化訊息顯示邏輯，修正 CJK 字元對齊與亂碼問題。
- **Scripting**: 改進 `ikkoku_quest.lua` 與 `yotsuya.lua` 流程，提升任務自動化穩定性。
- **Logging**: 日誌檔案現在會正確儲存於 `logs/` 目錄，並支援 HTML 格式。

### Fixed
- 修復 `MudNav` 在 Recovery 狀態下重複發送指令的問題。
- 修復 Session 關閉時網路連線未正確斷開的問題。
- 修復特定中文字串 (如 "七彩蓮花座") 顯示錯誤問題。

## [0.1.0] - 2026-01-01
### Added
- 專案初始化 (MVP)。
- 基礎 Telnet 連線與 Big5 支援。
- 核心 Lua 腳本引擎 integration.
- 基礎 UI 實作 (egui)。
