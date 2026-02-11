# MUD Client (mudclient-rs)

一個使用 Rust 開發的高效能、跨平台 MUD 客戶端。

## 特色功能

- **多語言支援**：穩定處理 Big5 編碼，完美顯示中文
- **ANSI 顏色**：完整解析 256 色與 TrueColor
- **別名系統 (Alias)**：命令縮寫與參數展開（如 `kk $1` → `kill $1;loot`）
- **觸發器系統 (Trigger)**：正則表達式匹配、自動發送命令、Lua 腳本執行
- **Lua 腳本引擎**：內嵌 Lua 5.4，支援進階自動化邏輯
- **多視窗管理**：將聊天、戰鬥等不同訊息路由到獨立子視窗
- **路徑記錄與迴圈偵測**：自動記錄移動路徑，偵測迷宮迴圈
- **日誌記錄**：純文字與 HTML 格式日誌，顏色完美重現
- **Tab 補齊**：畫面上的 Mob 名稱智慧補齊
- **自動重連**：斷線後自動嘗試重新連線
- **Profile 管理**：多角色設定檔、自動登入

## 下載

到 [Releases](https://github.com/suzuke/mudclient-rs/releases) 頁面下載預編譯的執行檔，或從 [Actions](https://github.com/suzuke/mudclient-rs/actions) 下載最新建置。

支援平台：
- **macOS** (Apple Silicon)
- **Windows** (x86_64)

## 從原始碼編譯

### 前置要求
- [Rust](https://rustup.rs/) (最新穩定版)

### 編譯與執行
```bash
cargo build -p mudgui --release
```

執行檔將產生在 `target/release/mudgui`。
將 `scripts/` 資料夾放在執行檔旁邊即可使用所有腳本功能。

## 目錄結構

```
mudclient-rs/
├── crates/
│   ├── mudcore/     # 核心：協定、編碼、別名、觸發器、Lua 腳本引擎
│   └── mudgui/      # GUI：基於 egui 的跨平台圖形介面
├── scripts/         # Lua 腳本（隨執行檔一起部署）
└── docs/            # 文件
```

## Lua 腳本 API

在觸發器或別名中使用 Lua 腳本，或用 `/lua` 指令直接執行：

```lua
mud.send("north")               -- 發送指令到伺服器
mud.echo("Hello!")               -- 本地顯示訊息
mud.log("記錄")                  -- 寫入日誌
mud.window("chat", text)         -- 輸出到子視窗
mud.timer(2.5, "mud.send('hi')") -- 延遲執行
mud.gag_message()                -- 攔截當前行
mud.enable_trigger("name", true) -- 啟用/禁用觸發器
```

更多細節請參考 [Scripting_and_Commands.md](docs/Scripting_and_Commands.md)。

## 快捷鍵

| 按鍵 | 功能 |
|------|------|
| Tab | 智慧補齊（Mob 名稱 / 歷史指令） |
| ↑ / ↓ | 瀏覽歷史指令 |
| Escape | 關閉彈出視窗 |
| F2-F4 | 開啟設定中心 |

## 授權

MIT License
