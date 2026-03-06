# 指令與腳本指南 (Scripting and Commands Guide)

本文件說明 mudclient-rs 客戶端支援的指令輸入與 Lua 腳本功能。

## 客戶端指令 (Client Commands)

這些指令可以直接在輸入框中使用，以 `#` 或 `/` 開頭。

### 1. 迴圈執行 (`#loop`)
重複執行指定的指令。

*   **語法**: `#loop <次數> <指令>`
*   **範例**:
    *   `#loop 3 smile` (執行 smile 3 次)
    *   `#loop 5 get all from corpse` (從屍體拿取東西 5 次)

### 2. 延遲執行 (`#delay`)
在指定的毫秒數後執行指令。

*   **語法**: `#delay <毫秒> <指令>`
*   **範例**:
    *   `#delay 1000 look` (1 秒後執行 look)
    *   `#delay 500 n;e;s` (0.5 秒後執行 n, e, s)

### 3. 執行 Lua 代碼 (`/lua`)
直接在命令行執行一行 Lua 代碼。

*   **語法**: `/lua <代碼>`
*   **範例**:
    *   `/lua mud.echo("Hello from Lua!")`
    *   `/lua mud.send("say 目前時間: " .. os.date())`

### 4. 變數操作 (`#var`, `#unvar`)
設定或刪除持久化變數（這些變數可以在觸發器、別名和 Lua 腳本中通過 `variables` 表訪問，或在指令中使用 `$varname`）。

*   **設定變數**: `#var <名稱> <值>`
    *   範例: `#var target big_monster`
*   **刪除變數**: `#unvar <名稱>`
    *   範例: `#unvar target`
*   **使用變數**: 在指令中若是 `$名稱` 會被替換。
    *   範例: `kill $target` (若 target 為 big_monster，則發送 `kill big_monster`)

### 5. 路徑與移動 (`#path`)
內建的路徑記錄與自動移動功能。

*   **語法**: `#path <子指令>`
*   **子指令**:
    *   `start` / `record`: 開始記錄移動路徑。
    *   `stop`: 停止記錄。
    *   `show`: 顯示目前記錄的路徑。
    *   `back`: 自動沿著原路返回 (Backtrack)。
    *   `clear`: 清除目前路徑。
    *   `undo`: 刪除上一步記錄。
    *   `save <名稱>`: 將目前路徑儲存到 Profile 中。
    *   `simplify` / `optimize`: 優化路徑（合併重複移動）。
    *   `loop <on|off>`: 開啟/關閉迴圈偵測功能。

---

## Lua 腳本 API (Lua Scripting API)

在觸發器 (Triggers)、別名 (Aliases) 的腳本模式中，或使用 `/lua` 時，可以使用 `mud` 物件與客戶端互動。

> 💡 **進階閱讀：**關於觸發器與別名的完整使用方式（包含設定格式、正則捕獲、參數替換等），請參考 [**觸發器與別名指南 (Triggers & Aliases)**](Triggers_Aliases.md)。

### `mud` 物件函數

| 函數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `mud.send(command)` | 發送指令到伺服器 | `mud.send("look")` |
| `mud.echo(text)` | 在主視窗顯示訊息 (不會發送到伺服器) | `mud.echo("腳本執行中...")` |
| `mud.log(message)` | 寫入訊息到系統日誌 | `mud.log("偵測到 Boss 出現")` |
| `mud.gag_message()` | 攔截當前行，不顯示在視窗中 (通常用於觸發器) | `mud.gag_message()` |
| `mud.window(name, text)` | 將訊息輸出到指定的子視窗 | `mud.window("chat", "頻道訊息...")` |
| `mud.timer(seconds, code)`| 設定延遲執行 (單位: 秒) | `mud.timer(2.5, "mud.send('heal')")` |
| `mud.enable_trigger(name, bool)`| 啟用或禁用指定名稱的觸發器 | `mud.enable_trigger("autoloot", false)` |
| `mud.get_room_id()` | 取得目前房間的 Hash ID | `local id = mud.get_room_id()` |
| `mud.get_current_room()` | 取得目前房間名稱 | `local name = mud.get_current_room()` |
| `mud.get_current_room_id()` | 取得目前房間 ID（同 get_room_id） | `local id = mud.get_current_room_id()` |
| `mud.collect_response(N)` | 收集接下來 N 行伺服器回應 | `mud.collect_response(10)` |
| `mud.ask_llm(prompt, callback)` | 非同步呼叫 LLM，回覆後執行 callback | 見下方範例 |

### 變數與表格

*   **`variables`**: 全域變數表 (Table)。
    *   讀取: `local t = variables["target"]`
    *   寫入: `variables["target"] = "orc"`
*   **`matches`** (或 `captures`): 觸發器的正則表達式捕獲組 (Captures)。
    *   `captures[1]` 代表第一個括號捕捉到的內容。
*   **`message`**: 當前觸發的原始訊息行。

### LLM 整合 (`mud.ask_llm`)

`mud.ask_llm(prompt, callback_lua_code [, model])` 可非同步呼叫 Anthropic API，收到回覆後執行 callback。callback 中用 `$RESULT` 代表 LLM 的回覆文字。

需設定環境變數 `ANTHROPIC_API_KEY`。預設使用 `claude-haiku-4-5-20251001`。

```lua
-- 詢問 LLM 並將回覆顯示在畫面
mud.ask_llm("說 hello", "mud.echo($RESULT)")

-- 在觸發器中使用：偵測到怪物時詢問 LLM 決策
mud.ask_llm(
  "我在「" .. room .. "」遇到「" .. mob .. "」，該打還是跑？只回答 kill 或 flee",
  "mud.send($RESULT)"
)

-- 指定模型
mud.ask_llm("翻譯這段文字", "mud.echo($RESULT)", "claude-sonnet-4-5-20250514")
```

## 範例腳本

以下是一個綜合範例，展示如何編寫 Lua 腳本來自動化操作。

```lua
-- 自動治療腳本範例
-- 假設觸發器捕捉到: "你的生命值還剩下 (100)/500"
-- 正則表達式: 你的生命值還剩下 (\d+)/(\d+)

local current_hp = tonumber(captures[1])
local max_hp = tonumber(captures[2])

if current_hp < (max_hp * 0.3) then
    mud.echo("⚠️ 生命危急！自動喝藥水...")
    mud.send("drink health_potion")
    
    -- 2秒後檢查是否需要再喝
    mud.timer(2.0, [[
        if variables["auto_heal"] == "true" then
            mud.send("score") -- 檢查狀態
        end
    ]])
else
    mud.echo("生命值安全: " .. current_hp)
end
```

---

## 事件系統 (Event System)

事件系統允許你監聽和發送自訂或內建事件，實現鬆耦合的腳本架構。

### 內建事件

| 事件名稱 | 觸發時機 | `event_data` 內容 |
| :--- | :--- | :--- |
| `connected` | 連線到伺服器 | `nil` |
| `disconnected` | 與伺服器斷線 | `nil` |
| `room_changed` | 房間變更 | 房間名稱 (string) |
| `command_sent` | 發送指令 | 指令文字 (string) |
| `state_changed` | 狀態機狀態變更 | JSON string `{"machine":"name","from":"old","to":"new"}` |

### API 函數

| 函數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `mud.on(event, code, [priority])` | 註冊事件處理器，回傳 handler_id | `local id = mud.on("connected", "mud.echo('hi')")` |
| `mud.once(event, code, [priority])` | 註冊一次性事件處理器 | `mud.once("room_changed", "mud.echo(event_data)")` |
| `mud.off(handler_id)` | 取消事件處理器 | `mud.off(id)` |
| `mud.emit(event, [data])` | 發送事件，data 可為 string 或 table | `mud.emit("combat_start", {target="orc"})` |

事件處理器的 code 中可透過全域變數 `event_data` 取得事件資料（JSON string 或 nil）。`priority` 為數字，數字越小越先執行，預設為 0。

```lua
-- 監聽房間變更
local id = mud.on("room_changed", [[
    mud.echo("進入房間: " .. (event_data or "unknown"))
]], 10)

-- 一次性監聽
mud.once("connected", "mud.echo('首次連線!')")

-- 發送自訂事件（table 會序列化為 JSON）
mud.emit("loot_found", { item = "sword", value = 100 })

-- 取消監聽
mud.off(id)
```

---

## 觸發器群組 (Trigger Groups)

觸發器可以指定 `group` 欄位來分組，並透過 API 批量啟用或禁用整個群組。

| 函數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `mud.enable_group(group, enabled)` | 啟用/禁用群組內所有觸發器 | `mud.enable_group("combat", false)` |

```lua
-- 禁用 combat 群組的所有觸發器
mud.enable_group("combat", false)

-- 啟用 combat 群組
mud.enable_group("combat", true)
```

觸發器定義中加入 `group` 欄位即可歸類：

```json
{
  "name": "auto_attack",
  "pattern": "怪物出現了",
  "script": "mud.send('kill monster')",
  "group": "combat"
}
```

---

## 狀態機 (State Machine)

狀態機提供有限狀態自動機 (FSM) 功能，可用於管理複雜的多步驟流程（如自動練功、任務腳本等）。

### API 函數

| 函數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `mud.state_machine(name, def)` | 建立狀態機 | 見下方範例 |
| `mud.sm_current(name)` | 取得目前狀態（string 或 nil） | `local s = mud.sm_current("bot")` |
| `mud.sm_transition(name, event)` | 手動觸發狀態轉換 | `mud.sm_transition("bot", "start")` |
| `mud.sm_reset(name)` | 重置為初始狀態 | `mud.sm_reset("bot")` |

### 定義格式

```lua
mud.state_machine("bot", {
    initial = "idle",
    states = {
        idle = {
            enter = "mud.echo('進入閒置')",   -- 進入狀態時執行
            exit  = "mud.echo('離開閒置')",   -- 離開狀態時執行
        },
        fighting = {
            enter = "mud.send('kill monster')",
            timeout = { seconds = 60, goto = "idle" },  -- 超時自動轉換
        },
    },
    transitions = {
        { from = "idle",     event = "engage",     to = "fighting" },
        { from = "fighting", event = "combat_end", to = "idle" },
    },
})

-- 查詢狀態
mud.echo(mud.sm_current("bot"))  -- "idle"

-- 觸發轉換
mud.sm_transition("bot", "engage")  -- idle -> fighting

-- 重置
mud.sm_reset("bot")  -- 回到 idle
```

狀態轉換時會自動發送 `state_changed` 事件，可搭配事件系統使用。

---

## 按鍵綁定 (Key Bindings)

將快捷鍵綁定到 Lua 代碼，支援功能鍵和組合鍵。

| 函數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `mud.bind_key(combo, code)` | 綁定按鍵 | `mud.bind_key("f5", "mud.send('look')")` |
| `mud.unbind_key(combo)` | 取消按鍵綁定 | `mud.unbind_key("f5")` |

支援的按鍵格式：`f1`~`f12`、`ctrl+<key>`、`alt+<key>`、`shift+<key>`，以及組合如 `ctrl+shift+a`。

```lua
-- 綁定 F5 為查看狀態
mud.bind_key("f5", "mud.send('score')")

-- 綁定 Ctrl+1 為快速治療
mud.bind_key("ctrl+1", "mud.send('drink heal_potion')")

-- 綁定 Alt+A 為自動攻擊
mud.bind_key("alt+a", "mud.send('kill $target')")

-- 取消綁定
mud.unbind_key("f5")
```

---

## 訊息路由 (Message Routing)

將符合特定模式的訊息自動導向子視窗，可選擇是否在主視窗中隱藏（gag）。

| 函數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `mud.add_route(def)` | 新增路由規則 | 見下方範例 |
| `mud.remove_route(name)` | 移除路由規則 | `mud.remove_route("chat")` |

路由定義欄位：

| 欄位 | 類型 | 說明 |
| :--- | :--- | :--- |
| `name` | string | 路由名稱（唯一識別） |
| `pattern` | string | 匹配模式（正則表達式） |
| `window` | string | 目標子視窗名稱 |
| `gag` | boolean | 是否從主視窗隱藏（預設 false） |

```lua
-- 將閒聊頻道導向 chat 視窗
mud.add_route({ name = "chat", pattern = "【閒聊】", window = "chat", gag = false })

-- 將系統訊息導向 system 視窗，並從主視窗隱藏
mud.add_route({ name = "system", pattern = "\\[System\\]", window = "system", gag = true })

-- 移除路由
mud.remove_route("chat")
```

---

## Debug 面板

側邊欄新增 **Debug** 頁籤，提供以下功能：

*   **Lua Console**: 即時執行 Lua 代碼並查看輸出
*   **Variables**: 查看與編輯全域變數表
*   **State Machines**: 查看所有狀態機的目前狀態
*   **Key Bindings**: 查看已綁定的快捷鍵
*   **Route Rules**: 查看已設定的訊息路由規則
*   **Event Log**: 查看最近的事件觸發記錄

---

## 相關文件

* [觸發器與別名指南 (Triggers & Aliases)](Triggers_Aliases.md)
* [腳本開發指南 (Script Guide)](ScriptGuide.md)
* [腳本使用手冊 (Scripts)](Scripts.md)
