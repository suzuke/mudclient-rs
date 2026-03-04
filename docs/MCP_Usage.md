# MCP Server 使用指南 (Model Context Protocol Usage)

`mudclient-rs` 包含兩層 AI 整合介面：

1. **內建 HTTP API** — 輕量級 REST API（axum），供外部程式直接控制客戶端
2. **Rust MCP Server** — 標準 MCP 協定（rmcp, stdio 模式），供 Claude Code / Claude Desktop 等 AI Agent 使用

## 一、HTTP API Server

### 啟動方式

HTTP API 預設隨 MUD 客戶端一同啟動。
*   **預設位址**: `http://127.0.0.1:9527`
*   **設定**: 可透過環境變數或啟動參數調整（目前固定為 9527）。

### API 端點總覽

| Method | Route | 說明 |
| :----- | :---- | :--- |
| GET | `/api/sessions` | 列出所有連線 session |
| GET | `/api/status` | 取得目前連線狀態 |
| GET | `/api/messages` | 獲取遊戲訊息（`?count=N`） |
| DELETE | `/api/messages` | 清空訊息緩衝區 |
| GET | `/api/room` | 取得目前房間資訊 |
| POST | `/api/send` | 發送指令到遊戲 |
| POST | `/api/lua` | 執行 Lua 腳本（fire-and-forget） |
| POST | `/api/evaluate` | 同步評估 Lua 腳本並回傳結果 |
| GET | `/api/aliases` | 列出所有 alias |
| GET | `/api/triggers` | 列出所有 trigger |
| GET | `/api/paths` | 列出所有 speedwalk path |
| GET | `/api/windows` | 列出所有子視窗 |
| GET | `/api/window/{id}` | 讀取特定視窗訊息（`?count=50`） |
| GET | `/api/history` | 指令歷史（`?count=50`） |
| GET | `/api/map/stats` | 地圖統計（房間數、邊數、啟用狀態） |
| GET | `/api/map/search` | 搜尋房間（`?query=`） |
| POST | `/api/map/path` | BFS 尋路 |
| POST | `/api/alias` | 管理 alias（add/remove/toggle） |
| POST | `/api/trigger` | 管理 trigger（add/remove/toggle） |

### 端點詳細說明

#### 執行 Lua 腳本 (`POST /api/lua`)

發送 Lua 代碼到客戶端執行（fire-and-forget，不等待結果）。

```bash
curl -X POST http://127.0.0.1:9527/api/lua \
  -H 'Content-Type: application/json' \
  -d '{"code":"mud.send(\"look\")"}'
```

#### 同步評估 Lua 腳本 (`POST /api/evaluate`)

發送 Lua 代碼到客戶端執行，並等待回傳結果。

```bash
curl -X POST http://127.0.0.1:9527/api/evaluate \
  -H 'Content-Type: application/json' \
  -d '{"code":"return mud.get_current_room_id()"}'
```

回應：
```json
{"ok": true, "message": "\"d41d8cd98f00b204e9800998ecf8427e\""}
```

#### 獲取遊戲訊息 (`GET /api/messages`)

```bash
curl -s 'http://127.0.0.1:9527/api/messages?count=20'
```

#### 清空訊息緩衝區 (`DELETE /api/messages`)

```bash
curl -X DELETE http://127.0.0.1:9527/api/messages
```

#### 列出 Aliases (`GET /api/aliases`)

```bash
curl -s http://127.0.0.1:9527/api/aliases
```

#### 列出 Triggers (`GET /api/triggers`)

```bash
curl -s http://127.0.0.1:9527/api/triggers
```

#### 列出 Paths (`GET /api/paths`)

```bash
curl -s http://127.0.0.1:9527/api/paths
```

#### 列出子視窗 (`GET /api/windows`)

```bash
curl -s http://127.0.0.1:9527/api/windows
```

#### 讀取子視窗訊息 (`GET /api/window/{id}`)

```bash
curl -s 'http://127.0.0.1:9527/api/window/chat?count=50'
```

#### 指令歷史 (`GET /api/history`)

```bash
curl -s 'http://127.0.0.1:9527/api/history?count=50'
```

#### 地圖統計 (`GET /api/map/stats`)

```bash
curl -s http://127.0.0.1:9527/api/map/stats
```

#### 搜尋房間 (`GET /api/map/search`)

```bash
curl -s 'http://127.0.0.1:9527/api/map/search?query=廣場'
```

#### BFS 尋路 (`POST /api/map/path`)

```bash
curl -X POST http://127.0.0.1:9527/api/map/path \
  -H 'Content-Type: application/json' \
  -d '{"from":"current","to":"目標房間名稱"}'
```

`from` 可用 `"current"` 表示目前所在房間。

#### 管理 Alias (`POST /api/alias`)

```bash
# 新增
curl -X POST http://127.0.0.1:9527/api/alias \
  -H 'Content-Type: application/json' \
  -d '{"action":"add","name":"go","pattern":"^go (.+)","replacement":"walk $1"}'

# 刪除
curl -X POST http://127.0.0.1:9527/api/alias \
  -H 'Content-Type: application/json' \
  -d '{"action":"remove","name":"go"}'

# 啟用/停用
curl -X POST http://127.0.0.1:9527/api/alias \
  -H 'Content-Type: application/json' \
  -d '{"action":"toggle","name":"go"}'
```

#### 管理 Trigger (`POST /api/trigger`)

```bash
# 新增
curl -X POST http://127.0.0.1:9527/api/trigger \
  -H 'Content-Type: application/json' \
  -d '{"action":"add","name":"autoheal","pattern":"生命值低於","pattern_type":"contains","script":"mud.send(\"heal\")"}'

# 刪除 / 啟用停用同 alias
```

---

## 二、Rust MCP Server

### 概覽

`mudclient-mcp` 是 Rust 原生的 MCP Server（基於 rmcp 1.x），透過 stdio 與 AI Agent 通訊，內部呼叫 HTTP API 存取 MUD 客戶端。

### 安裝與設定

```bash
# 編譯
cargo build -p mcp-server --release

# 執行檔位於
target/release/mudclient-mcp
```

### Claude Code 設定

在 `.claude/settings.json` 中加入：

```json
{
  "mcpServers": {
    "mudclient": {
      "command": "/path/to/mudclient-mcp",
      "env": {
        "MUDCLIENT_API_URL": "http://127.0.0.1:9527"
      }
    }
  }
}
```

### Claude Desktop 設定

在 `claude_desktop_config.json` 中加入：

```json
{
  "mcpServers": {
    "mudclient": {
      "command": "/path/to/mudclient-mcp",
      "env": {
        "MUDCLIENT_API_URL": "http://127.0.0.1:9527"
      }
    }
  }
}
```

### 19 個 MCP 工具

| 工具名稱 | 說明 |
| :------- | :--- |
| `list_sessions` | 列出所有連線 session |
| `get_status` | 取得連線狀態（角色、伺服器、房間等） |
| `read_messages` | 讀取最近 N 行遊戲訊息 |
| `clear_messages` | 清空訊息緩衝區 |
| `get_room_info` | 取得目前房間詳細資訊 |
| `send_command` | 發送指令到遊戲伺服器 |
| `execute_lua` | 執行 Lua 腳本（無回傳值） |
| `evaluate_lua` | 同步評估 Lua 並回傳結果 |
| `get_map_stats` | 地圖資料庫統計 |
| `search_map_rooms` | 依名稱搜尋房間 |
| `find_map_path` | BFS 尋路（支援 "current" 起點） |
| `list_aliases` | 列出所有 alias |
| `list_triggers` | 列出所有 trigger |
| `list_paths` | 列出所有 speedwalk path |
| `manage_alias` | 新增/刪除/啟停 alias |
| `manage_trigger` | 新增/刪除/啟停 trigger |
| `list_windows` | 列出所有子視窗 |
| `read_window` | 讀取特定子視窗訊息 |
| `get_command_history` | 取得指令歷史 |

---

## 整合應用場景

1.  **AI Agent 整合**: Claude Code 透過 MCP 工具直接操控 MUD 角色，讀取訊息、發送指令、管理觸發器。
2.  **外部監控**: 編寫 Python 腳本透過 HTTP API 監控血量或特定訊息，並觸發警報。
3.  **自動化測試**: 透過 API 發送測試指令並驗證回應，無需人工介入。
4.  **LLM 觸發器**: 在 Lua 觸發器中使用 `mud.ask_llm()` 呼叫 Anthropic API，讓 LLM 即時做出遊戲決策。

## 常見問題

*   **Q: 為什麼 curl 回傳 "Failed to connect"?**
    *   A: 請確認 MUD 客戶端是否已啟動且正在執行。
*   **Q: 執行 Lua 出錯怎麼辦?**
    *   A: API 會回傳 `{"ok": false, "error": "錯誤訊息"}`，請檢查 Lua 語法。
*   **Q: MCP Server 的環境變數?**
    *   A: `MUDCLIENT_API_URL` 預設為 `http://127.0.0.1:9527`，可指向不同的 API 位址。
