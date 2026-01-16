# Rust MUD Client

一個使用 Rust 開發的高效能、功能豐富的 MUD 客戶端。

## 特色功能

- **多語言支援**: 穩定處理 Big5 編碼，完美顯示中文。
- **ANSI 顏色**: 完整解析並顯示 MUD 中的 ANSI 顏色碼。
- **別名系統 (Alias)**: 支援命令縮寫與參數展開（例如：`kk $1` -> `kill $1;loot`）。
- **觸發器系統 (Trigger)**: 正則表達式匹配、自動高亮、聲音通知、自動發送命令與訊息抑制 (Gag)。
- **Python 腳本**: 內嵌 Python 引擎，支援使用 Python 撰寫進階自動化邏輯。
- **多視窗管理**: 支援將不同的訊息（如聊天、戰鬥）路由到獨立的子視窗。
- **日誌記錄**: 支援純文字、原始與 HTML 格式的日誌存儲，顏色外觀完美重現。

## 下載與安裝

### 前置要求
- [Rust](https://rustup.rs/) (最新穩定版)
- [Miniconda](https://docs.conda.io/en/latest/miniconda.html) 或 Python 3.11+ (腳本功能需要)

### 編譯
```bash
cargo build --release
```

## 使用指南

### 管理中心
點擊側邊欄的 **「中心管理」** 按鈕即可進入管理介面，檢視當前已載入的：
- 別名 (Alias)
- 觸發器 (Trigger)
- 日誌狀態 (Logger)

### Python 腳本 API
在腳本中可以訪問 `mud` 物件：
```python
mud.send("north")             # 發送指令
mud.set_var("hp", "500")      # 設置變數
mud.gag_message()             # 抑制當前訊息
print(message)                # 訪問原始訊息
print(captures[0])            # 訪問正則匹配群組
```

## 開發架構

- `mudcore`: 核心協定、編碼、別名、觸發器與腳本邏輯。
- `mudgui`: 基於 `egui` 的跨平台圖形介面。

---

## 授權
MIT License
