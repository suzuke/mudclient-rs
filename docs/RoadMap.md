# 專案路線圖 (Roadmap)

## 📌 當前狀態
目前專案處於 **v0.7.3** 穩定版。核心功能（連線、腳本、觸發器、別名、地圖、MCP Server、內嵌終端）均已完成，持續進行 UI 優化與進階功能開發。

## 📅 里程碑

### v0.1.0 (MVP) - ✅ 已完成 (2026-02-12)
- [x] 基礎 Telnet 連線 (Big5 編碼支援)
- [x] ANSI 顏色與格式解析 (256色/TrueColor)
- [x] 核心指令系統 (`#loop`, `#delay`, `#var`)
- [x] 基礎觸發器 (Trigger) 與別名 (Alias)
- [x] Lua 腳本引擎整合 (`mud.*` API)
- [x] 多視窗/分頁顯示
- [x] macOS .app bundle 打包

### v0.1.x (快速迭代) - ✅ 已完成 (2026-02-12 ~ 2026-03-03)
- [x] Big5/ANSI 狀態機穩健處理 (跨封包)
- [x] CJK 字元對齊修正 (半寬/全寬符號置中)
- [x] 標點符號正規化 (逗號、句號、全形統一)
- [x] IME 輸入法組字修正 (Windows)
- [x] 房間偵測改進 (NPC/玩家/系統訊息過濾)
- [x] Tab 自動補齊增強 (指令字典、Prompt 排除)
- [x] Numpad 方向鍵支援 (含 egui patch)
- [x] 字型大小調整 (10-24px slider, ⌘+/-/0)
- [x] Multi-Session MCP 支援
- [x] Lua 模組系統 (MudNav, MudCombat, MudLoot, MudMapper, MudExplorer)
- [x] 任務腳本庫 (ikkoku_quest, poker_quest, smurf_quest, itemfarm)
- [x] macOS Universal Binary 支援
- [x] 觸發器與別名 UI 改進

### v0.5.0 (穩定化) - ✅ 已完成 (2026-03-04)
- [x] Rust 原生 MapDatabase (取代 Lua MudMapper 資料庫)
- [x] Toast UI 通知回饋 (Profile export/import)
- [x] 全面性程式碼品質改進 (P0/P1/P2 refactoring)
- [x] 工具列三列佈局恢復
- [x] Telnet prompt boundary 房間偵測

### v0.5.1 (修復) - ✅ 已完成 (2026-03-04)
- [x] Password Storage 改為 base64-encoded JSON (跨平台相容)

### v0.6.0 (AI 整合) - ✅ 已完成 (2026-03-05)
- [x] Rust 原生 MCP Server (rmcp 1.x, stdio, 19 個工具)
- [x] HTTP API 擴充 (11 個新端點，共 19 個 REST 端點)
- [x] `mud.ask_llm()` Lua API (觸發器非同步呼叫 Anthropic LLM)
- [x] Mapper 改進 (auto-save, 碰撞處理, 方向標籤)
- [x] 修復 Mapper 導致的輸入延遲 (5 項針對性優化)

### v0.7.0 (內嵌終端) - ✅ 已完成 (2026-03-05)
- [x] 側邊欄內嵌終端模擬器 (`egui-term` crate)

### v0.7.1 ~ v0.7.3 (修復與優化) - ✅ 已完成 (2026-03-05 ~ 2026-03-06)
- [x] 修正房間 hash ID (hash 演算法變更後更新)
- [x] 修正 MudNav 過期 timeout timer 導致 false Step Stuck
- [x] macOS app icon 修正 (icon.png 轉換為真實 PNG 格式)
- [x] README、CHANGELOG、Architecture 文件同步

---

### v0.8.0 (UI 完善) - 📝 規劃中
- [ ] 字型設定 — 允許使用者自訂字型家族（大小已可調）
- [ ] 主題切換 — 深色/淺色模式切換

### v0.9.0 (進階擴展) - 📝 規劃中
- [ ] 插件系統架構設計
- [ ] 自動導航 (Pathfinding) Rust 原生整合 — Lua BFS 版已可用，視效能需求決定
- [ ] 無障礙支援 — 螢幕閱讀器相容性優化

### v1.0.0 (長期願景) - 🔮 構想中
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
- [x] **側邊欄**：整合筆記、指南、工具與內嵌終端
- [ ] **字型設定**：允許使用者自訂字型與大小 (目前寫死)
- [ ] **主題切換**：深色/淺色模式微調

### MCP / AI 自動化
- [x] **REST API**：內建 Axum HTTP Server（19 個端點），供外部工具存取
- [x] **MCP Server**：Rust 原生（rmcp 1.x），19 個工具，支援 AI Agent 控制 MUD
- [x] **Multi-Session API**：透過 session key 指定操作目標
- [x] **Lua 執行**：遠端執行/評估 Lua 程式碼
- [x] **LLM 整合**：`mud.ask_llm()` 讓 Lua 觸發器非同步呼叫 Anthropic API

### 進階功能 (Advanced)
- [x] **路徑系統**：路徑錄製 (`#path record`)、倒帶 (`#path back`)
- [x] **地圖資料**：MudMapper 房間識別 (Hash/ID)、MudNav 導航
- [x] **地圖視覺化**：egui canvas 渲染、BFS 佈局、zoom/pan 互動
- [x] **自動導航**：Lua BFS pathfinding (`MudMapper.find_path`)
- [x] **內嵌終端**：側邊欄終端模擬器 (egui-term)

> 最後更新: 2026-03-06
