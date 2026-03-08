# Lua API 參考手冊

> **適用版本**：mudclient-rs v0.8+
> **最後更新**：2026-03-08

本文件列出所有 `mud.*` API 及內建模組的完整說明。

---

## 目錄

1. [IDE 設定（LuaLS 補全）](#ide-設定)
2. [基本 I/O](#基本-io)
3. [計時器](#計時器)
4. [觸發器控制](#觸發器控制)
5. [非同步回應收集](#非同步回應收集)
6. [事件系統](#事件系統)
7. [狀態機](#狀態機)
8. [地圖/房間](#地圖房間)
9. [按鍵綁定](#按鍵綁定)
10. [訊息路由](#訊息路由)
11. [內建 json 模組](#內建-json-模組)
12. [全域變數與 Hook](#全域變數與-hook)
13. [持久化變數](#持久化變數)

---

## IDE 設定

專案已內建 LuaLS 型別定義。在 VSCode 安裝 [Lua (sumneko.lua)](https://marketplace.visualstudio.com/items?itemName=sumneko.lua) 擴展後，編輯 `scripts/` 目錄下的檔案即可享有：

- `mud.` 自動補全所有 API
- 參數型別與說明提示
- `json.encode` / `json.decode` 補全
- 全域 Hook 函數簽名提示

型別定義檔位於 `scripts/types/`，設定檔為 `.luarc.json`。

---

## 基本 I/O

### `mud.send(cmd)`

送出指令到 MUD 伺服器。

```lua
mud.send("kill goblin")
mud.send("say hello")
```

### `mud.echo(text)`

本地顯示訊息。支援 ANSI 色碼（`{r` 紅色、`{g` 綠色、`{G` 亮綠、`{x` 重置等）。

```lua
mud.echo("{G[系統] 戰鬥開始{x")
```

> `print()` 已被覆寫為 `mud.echo()`。

### `mud.window(name, text)`

輸出訊息到指定子視窗。

```lua
mud.window("combat", "{r戰鬥訊息{x")
```

### `mud.gag_message()`

抑制當前觸發的訊息，不顯示在主視窗。通常在觸發器或 `on_server_message` hook 中使用。

```lua
if clean:find("廣告") then
    mud.gag_message()
end
```

### `mud.log(msg)`

寫入日誌訊息（同時輸出到 Rust 層 tracing）。

### `mud.start_log(path)` / `mud.stop_log()`

開始/停止日誌記錄到檔案。

---

## 計時器

### `mud.timer(seconds, callback)`

延遲執行。`callback` 可以是**字串**或 **function**。

```lua
-- 字串模式（傳統，向下相容）
mud.timer(5, "mud.send('look')")

-- function 模式（推薦：有 closure 捕獲、IDE 補全、語法檢查）
local target = "dragon"
mud.timer(5, function()
    mud.send("kill " .. target)
end)

-- 支援小數秒
mud.timer(0.5, function()
    mud.echo("半秒後")
end)
```

**注意**：
- function 模式下，closure 正確捕獲外層變數
- 內部轉換為毫秒精度
- 建議搭配 `MudUtils.safe_timer()` 使用 run_id 防競態

---

## 觸發器控制

### `mud.enable_trigger(name, enabled)`

啟用或禁用指定觸發器。

```lua
mud.enable_trigger("auto_loot", true)
mud.enable_trigger("auto_loot", false)
```

### `mud.enable_group(group_name, enabled)`

啟用或禁用整個觸發器群組。

```lua
mud.enable_group("combat_triggers", false)
```

---

## 非同步回應收集

### `mud.collect_response(cmd, callback_code)`

送出指令並收集伺服器回應（直到下一個 prompt）。回應完成後執行 callback。

callback 中透過 `_G._collected_lines` 取得回應行陣列。

```lua
mud.collect_response("inventory", [[
    for _, line in ipairs(_G._collected_lines) do
        if line:find("sword") then
            mud.echo("{G找到劍了！{x")
        end
    end
]])
```

**原理**：Rust 網路層偵測到 prompt（incomplete line + 50ms timeout）後觸發 callback。不需在 Lua 端匹配 prompt pattern。

### `mud.send_chain(cmds, [callback_code])`

依序送出多個指令，每個等待回應後才發下一個。只有最後一個指令的回應觸發 callback。

```lua
mud.send_chain({"get key", "open door", "north"}, [[
    mud.echo("已通過門")
]])
```

### `mud.ask_llm(prompt, callback_code, [model])`

非同步呼叫 Claude API。callback 中 `$RESULT` 被替換為 LLM 回覆字串。

```lua
mud.ask_llm("翻譯成中文：hello world", [[
    mud.echo("翻譯結果: " .. $RESULT)
]])

-- 指定模型
mud.ask_llm("分析這段戰鬥", callback, "claude-sonnet-4-5-20241022")
```

**需要**：設定環境變數 `ANTHROPIC_API_KEY`。
**預設模型**：`claude-haiku-4-5-20251001`。

---

## 事件系統

### `mud.on(event_name, lua_code, [priority])` → handler_id

註冊持續觸發的事件處理器。priority 越大越先執行。

```lua
local id = mud.on("combat_end", "mud.send('loot all')")
```

### `mud.once(event_name, lua_code, [priority])` → handler_id

註冊一次性事件處理器（觸發一次後自動移除）。

```lua
mud.once("connected", "mud.send('look')", 10)
```

### `mud.off(handler_id)`

移除事件處理器。

```lua
mud.off(id)
```

### `mud.emit(event_name, [data])`

發射事件。data 會被 JSON 序列化後傳遞給處理器。

```lua
mud.emit("quest_complete", {quest = "smurf", time = 120})
```

---

## 狀態機

### `mud.state_machine(name, definition)`

定義並啟動狀態機。

```lua
mud.state_machine("my_sm", {
    initial = "idle",
    states = {
        idle = {
            enter = [[mud.echo("進入 idle")]],
            exit  = [[mud.echo("離開 idle")]],
        },
        running = {
            enter = [[mud.send("look")]],
            -- 超時設定：120 秒後自動轉到 idle
            timeout = { seconds = 120, target = "idle" },
        },
    },
    transitions = {
        { from = "idle",    event = "start", to = "running" },
        { from = "running", event = "done",  to = "idle" },
    },
})
```

### `mud.sm_current(name)` → string | nil

取得狀態機當前狀態。

### `mud.sm_transition(name, event)`

觸發狀態機轉移。

```lua
mud.sm_transition("my_sm", "start")
```

### `mud.sm_reset(name)`

重置狀態機到初始狀態。

### `mud.sm_remove(name)`

移除狀態機。

---

## 地圖/房間

### `mud.get_room_id(name, desc, exits, [strict])` → string

計算房間雜湊 ID。`strict=true`（預設）使用完整描述，`false` 僅用名稱+出口。

### `mud.get_current_room_id([strict])` → string | nil

取得當前所在房間 ID。

### `mud.get_current_room()` → table | nil

取得當前房間完整資訊。

```lua
local room = mud.get_current_room()
if room then
    mud.echo("房間: " .. room.name)
    mud.echo("出口: " .. table.concat(room.exits, ", "))
end
```

回傳格式：`{ name = "...", description = "...", exits = {"north", "south", ...} }`

---

## 按鍵綁定

### `mud.bind_key(key_combo, lua_code)`

綁定快捷鍵。

```lua
mud.bind_key("ctrl+f1", "mud.send('recall')")
mud.bind_key("alt+a", "mud.send('agg 100')")
```

### `mud.unbind_key(key_combo)`

解除快捷鍵綁定。

---

## 訊息路由

### `mud.add_route(definition)`

新增訊息路由規則，將符合 pattern 的訊息導向子視窗。

```lua
mud.add_route({
    name    = "chat",
    pattern = "【閒聊】",
    window  = "chat",
    gag     = false,  -- true 則同時隱藏主視窗
})
```

### `mud.remove_route(name)`

移除訊息路由規則。

---

## 內建 json 模組

全域可用，無需 require（也可以 `require("json")`）。

### `json.encode(value, [pretty])` → string

將 Lua 值編碼為 JSON 字串。

```lua
local s = json.encode({name = "武器", damage = 50, flags = {"magic", "glow"}})
-- => '{"damage":50,"flags":["magic","glow"],"name":"武器"}'

-- Pretty print（2-space indent）
local pretty = json.encode({hp = 100}, true)
-- => '{\n  "hp": 100\n}'
```

支援的型別：table（array/object 自動判斷）、string、number、boolean、nil（→ null）。

### `json.decode(str)` → any

將 JSON 字串解碼為 Lua 值。

```lua
local data = json.decode('{"hp":100,"items":["sword","shield"]}')
print(data.hp)        -- 100
print(data.items[1])  -- "sword"
```

JSON `null` 對應 Lua `nil`。

**取代自製 JSON**：現有腳本中的 `json_encode()` / `json_decode()` 可逐步替換為內建 `json` 模組。

---

## 全域變數與 Hook

### 觸發器/Hook 中可用的全域變數

| 變數 | 型別 | 說明 |
|------|------|------|
| `message` | string | 當前訊息（含 ANSI 色碼） |
| `clean_message` | string | 純淨訊息（已去 ANSI 和 `\r`） |
| `captures` | string[] | 觸發器正則捕獲組 |

### Hook 函數

使用者可定義的全域 Hook 函數（透過 `MudUtils.register_hook` 或直接定義）：

#### `on_server_message(msg, clean, is_echo)`

每收到一行伺服器訊息時觸發。

```lua
function on_server_message(msg, clean, is_echo)
    if clean:find("^你獲得") then
        mud.echo("{G[戰利品] " .. clean .. "{x")
    end
end
```

- `msg`：原始訊息（含 ANSI）
- `clean`：純淨訊息（已去 ANSI 和 `\r`，可直接用 `^` 錨定匹配）
- `is_echo`：是否為本地回顯

#### `on_command(cmd, clean_cmd, is_echo)`

使用者送出指令時觸發。

#### `on_room_detected(room_id, room_name, is_echo)`

偵測到新房間時觸發。

---

## 持久化變數

`mud.variables` 表中的值會自動持久化，跨腳本共享。

```lua
-- 設定
mud.variables.hp = "100"
mud.variables.target = "goblin"

-- 讀取（在其他腳本或觸發器中）
local hp = mud.variables.hp
```

支援 `$var` 語法在指令中展開：

```
/send kill $target
```

---

## 錯誤處理

所有 Lua 執行錯誤（觸發器、計時器、SM callback、事件處理器、Hook）都會在遊戲視窗中以 `[System]` 訊息顯示，同時寫入 Rust log。

使用 `pcall` 包裝可能出錯的程式碼：

```lua
local ok, err = pcall(function()
    -- 可能出錯的邏輯
end)
if not ok then
    mud.echo("{r[Error] " .. tostring(err) .. "{x")
end
```
