# 環境變數 (Environment Variables)

`mudclient-rs` 目前支援以下環境變數，主要用於開發除錯與日誌控制。

## 日誌控制 (Logging)

本專案使用 `tracing-subscriber` 處理日誌，支援標準的 `RUST_LOG` 過濾語法。

### `RUST_LOG`

控制輸出的日誌層級與模組。

*   **預設值**: `info` (若未設定)
*   **格式**: `target=level`
*   **範例**:
    *   `RUST_LOG=debug`: 顯示所有 debug 訊息。
    *   `RUST_LOG=mudcore=trace,mudgui=info`: 詳細顯示核心層訊息，但介面層只顯示 info。
    *   `RUST_LOG=off`: 關閉日誌。

## 執行範例

**Linux / macOS:**
```bash
RUST_LOG=debug ./mudgui
```

**Windows (PowerShell):**
```powershell
$env:RUST_LOG="debug"; ./mudgui.exe
```

## 設定檔 (Configuration)

除了環境變數，應用程式也會讀取 `.agent/config.yaml` (如果有) 或使用者目錄下的配置檔進行初始化。
