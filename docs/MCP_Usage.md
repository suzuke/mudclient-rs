# MCP Server 使用指南 (Model Context Protocol Usage)

`mudclient-rs` 內建了一個輕量級的 HTTP API Server，允許外部程式（如 AI Agent、自動化腳本、Stream Deck 等）直接控制 MUD 客戶端並獲取遊戲訊息。

## 啟動方式

MCP Server 預設隨 MUD 客戶端一同啟動。
*   **預設位址**: `http://127.0.0.1:9527`
*   **設定**: 可透過環境變數或啟動參數調整（目前固定為 9527）。

## API 端點 (Endpoints)

### 1. 執行 Lua 腳本 (`POST /api/lua`)

發送 Lua 代碼到客戶端執行。這是最強大的控制方式，可以調用所有客戶端支援的 Lua API（如 `mud.send`, `mud.echo` 等）。

*   **URL**: `http://127.0.0.1:9527/api/lua`
*   **Method**: `POST`
*   **Content-Type**: `application/json`
*   **Body**:
    ```json
    {
      "code": "你的 Lua 代碼"
    }
    ```

#### 範例 (curl)

**發送指令到遊戲:**
```bash
curl -X POST http://127.0.0.1:9527/api/lua \
  -H 'Content-Type: application/json' \
  -d '{"code":"mud.send(\"look\")"}'
```

**顯示訊息在視窗:**
```bash
curl -X POST http://127.0.0.1:9527/api/lua \
  -H 'Content-Type: application/json' \
  -d '{"code":"mud.echo(\"Hello from MCP!\")"}'
```

**重新載入所有模組 (Reload All):**
```bash
curl -X POST http://127.0.0.1:9527/api/lua \
  -H 'Content-Type: application/json' \
  -d '{"code":"package.loaded[\"scripts.modules.MudUtils\"] = nil; require(\"scripts.modules.MudUtils\"); mud.echo(\"Reloaded!\")"}'
```

### 2. 同步評估 Lua 腳本 (`POST /api/evaluate`)

發送 Lua 代碼到客戶端執行，並且**等待**並回傳執行結果的 JSON 字串表示。適用於需要即時讀取遊戲狀態或變數的場景。

*   **URL**: `http://127.0.0.1:9527/api/evaluate`
*   **Method**: `POST`
*   **Content-Type**: `application/json`
*   **Body**:
    ```json
    {
      "code": "return mud.get_current_room_id()"
    }
    ```

#### 範例 (curl)

**取得目前的房間 ID:**
```bash
curl -X POST http://127.0.0.1:9527/api/evaluate \
  -H 'Content-Type: application/json' \
  -d '{"code":"return mud.get_current_room_id()"}'
```

**回應 (`"ok": true` 搭配 JSON 化腳本結果):**
```json
{
  "ok": true,
  "message": "\"d41d8cd98f00b204e9800998ecf8427e\""
}
```

### 3. 獲取遊戲訊息 (`GET /api/messages`)

獲取 MUD 視窗中最新的遊戲訊息。支援 Polling。

*   **URL**: `http://127.0.0.1:9527/api/messages`
*   **Method**: `GET`
*   **Query Params**:
    *   `count`: (選填) 要獲取的訊息行數。預設 `10`，最大 `200`。

#### 範例 (curl)

**獲取最新 20 行:**
```bash
curl -s "http://127.0.0.1:9527/api/messages?count=20"
```

**回應格式:**
```json
{
  "messages": [
    "你正走在通往北方的賈不妙的城堡的小徑上。",
    "[出口: 北 南]",
    "小精靈的老爸(Papa Smurf)正在練習著魔法."
  ],
  "total": 3
}
```

### 4. 清空訊息緩衝區 (`DELETE /api/messages`)

清空 MUD 客戶端目前的訊息緩衝區。這在開始特定任務（例如戰鬥或對話）前非常有用，可以用來清除舊的無關訊息，確保之後讀取到的都是最新狀態，能大幅降低 AI 解析負擔。

*   **URL**: `http://127.0.0.1:9527/api/messages`
*   **Method**: `DELETE`

#### 範例 (curl)

```bash
curl -X DELETE http://127.0.0.1:9527/api/messages
```

**回應:**
```json
{
  "ok": true,
  "message": "Cleared 150 messages"
}
```

## 整合應用場景

1.  **AI Agent 整合**: Agent 可以透過 POST `/api/lua` 執行複雜操作，並透過 `/api/messages` 獲取執行結果（Observation）。
2.  **外部監控**: 編寫 Python 腳本監控血量或特定訊息，並觸發警報。
3.  **自動化測試**: 透過 API 發送測試指令並驗證回應，無需人工介入。

## 常見問題

*   **Q: 為什麼 curl 回傳 "Failed to connect"?**
    *   A: 請確認 MUD 客戶端是否已啟動且正在執行。
*   **Q: 執行 Lua 出錯怎麼辦?**
    *   A: API 會回傳 `{"ok": false, "error": "錯誤訊息"}`，請檢查 Lua 語法。
