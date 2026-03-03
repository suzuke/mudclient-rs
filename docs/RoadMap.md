# 專案路線圖 (Roadmap)

## 📌 當前狀態
目前專案處於 **Beta** 階段。核心功能（連線、腳本、觸發器、別名）已穩定，正在進行進階功能開發（自動地圖、更多任務腳本）與 UI 優化。

## 📅 里程碑

### v0.1.0 (MVP) - ✅ 已完成
- [x] 基礎 Telnet 連線 (Big5 編碼支援)
- [x] ANSI 顏色與格式解析
- [x] 核心指令系統 (`#loop`, `#delay`, `#var`)
- [x] 基礎觸發器 (Trigger) 與別名 (Alias)
- [x] Lua 腳本引擎整合 (`mud.*` API)
- [x] 多視窗/分頁顯示

### v0.2.0 (增強功能) - ✅ 已完成
- [x] 側邊欄工具面板 (Tools, Notes, Guide)
- [x] 日誌系統優化 (HTML 輸出, 訊息折疊)
- [x] 自動路徑記錄與回溯 (`#path`)
- [x] MCP Server (Node.js) — AI 外部控制 API
- [x] Multi-Session 支援 — 多連線管理與切換
- [x] 輸入自動補齊 (Autocomplete) — 房間描述、怪物/玩家 ID
- [x] Lua 模組系統 (MudNav, MudCombat, MudLoot, MudMapper, MudExplorer, MudUtils)
- [x] 任務腳本庫擴充 (ikkoku_quest, poker_quest, smurf_quest, itemfarm 等)

### v0.3.0 (穩定與發布) - 🏃 進行中
- [x] CI/CD 自動建置流程 (GitHub Actions)
- [x] macOS DMG 打包 (Apple Silicon / Intel Universal)
- [x] 文件補齊 (API、腳本教學、觸發器與別名指南)
- [ ] 跨平台執行檔簽章 (macOS Notarization / Windows Signing)
- [ ] 自動地圖繪製 (Mapper) 視覺化整合
- [ ] 使用者設定檔 (Profile) 管理介面優化

### v0.4.0 (進階擴展) - 📝 規劃中
- [ ] 插件系統架構設計
- [ ] 自動導航 (Pathfinding) 整合 MudMapper
- [ ] 無障礙支援 — 螢幕閱讀器相容性優化
- [ ] 字型設定 — 允許使用者自訂字型與大小

### v0.5.0 (長期願景) - 🔮 構想中
- [ ] 跨平台 Linux 支援 (AppImage / deb 打包)
- [ ] 雲端同步 — 設定檔、腳本、地圖跨裝置同步
- [ ] 社群腳本倉庫 — 線上瀏覽與安裝社群腳本
- [ ] 內建地圖編輯器 — 視覺化建圖與路線規劃

## 🎯 功能需求清單

### 核心功能 (Core)
- [x] **連線管理**：支援自動重連、多連線 (Multi-Session)
- [x] **顯示引擎**：256色/TrueColor 支援、CJK 字元對齊優化
- [x] **輸入處理**：指令歷史紀錄、Tab 補齊、Autocomplete
- [x] **日誌系統**：支援純文字與 HTML 格式，包含時間戳記與顏色

### 腳本與自動化 (Scripting)
- [x] **Lua 綁定**：完整 `mud.*` API (send, echo, trigger, timer)
- [x] **觸發器**：支援正則表達式 (Regex) 與 Lua 回調
- [x] **別名**：支援參數替換 (`$1`, `$2`)
- [x] **計時器**：一次性 (`tempTimer`) 與循環計時器
- [x] **模組化腳本**：`scripts/modules/` 共用邏輯 (MudNav, MudCombat 等)

### 使用者介面 (UI/UX)
- [x] **多視窗**：支援分頁 (Tabs) 切換
- [x] **側邊欄**：整合筆記、指南與工具
- [ ] **字型設定**：允許使用者自訂字型與大小 (目前寫死)
- [ ] **主題切換**：深色/淺色模式微調

### MCP / AI 自動化
- [x] **REST API**：內建 Axum HTTP Server，供外部工具存取
- [x] **MCP Server**：Node.js/TypeScript，支援 AI Agent 控制 MUD
- [x] **Multi-Session API**：透過 session key 指定操作目標
- [x] **Lua 執行**：遠端執行/評估 Lua 程式碼

### 進階功能 (Advanced)
- [x] **路徑系統**：路徑錄製 (`#path record`)、倒帶 (`#path back`)
- [x] **地圖資料**：MudMapper 房間識別 (Hash/ID)、MudNav 導航
- [ ] **地圖視覺化**：Canvas/Mermaid 渲染
- [ ] **自動導航**：Pathfinding 演算法整合

> 最後更新: 2026-03-02
