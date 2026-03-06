# 事件驅動系統使用教學

> **適用版本**：mudclient-rs v0.9.0+
> **最後更新**：2026-03-07

本文件介紹如何使用事件系統、觸發器群組、狀態機、快捷鍵綁定與訊息路由，從基礎概念到實戰整合。

---

## 目錄

1. [核心概念](#核心概念)
2. [事件系統基礎](#事件系統基礎)
3. [觸發器群組](#觸發器群組)
4. [狀態機](#狀態機)
5. [快捷鍵綁定](#快捷鍵綁定)
6. [訊息路由](#訊息路由)
7. [Debug 面板](#debug-面板)
8. [實戰範例：自動練功 Bot](#實戰範例自動練功-bot)
9. [常見問題](#常見問題)

---

## 核心概念

傳統的觸發器是「文字匹配 → 執行動作」的一對一映射。事件系統在此基礎上增加了一層**解耦**：

```
觸發器 (文字匹配)
    ↓ mud.emit("combat_end")
事件匯流排 (EventBus)
    ├→ Event Handler A (撿東西)
    ├→ Event Handler B (記錄戰鬥)
    └→ State Machine (切換到 looting 狀態)
```

好處：
- **一個事件，多個響應**：不用在一個觸發器裡塞所有邏輯
- **模組解耦**：戰鬥模組只管發 `combat_end`，撿取模組只管聽 `combat_end`
- **狀態管理**：複雜流程用狀態機管理，不再靠一堆 flag 變數

---

## 事件系統基礎

### 監聽事件

```lua
-- 監聽事件，返回 handler ID（用於之後取消）
local id = mud.on("事件名稱", "要執行的 Lua 程式碼")

-- 帶優先序（數字越小越先執行，預設 0）
mud.on("combat_end", "mud.send('get all from corpse')", -10)  -- 最先執行
mud.on("combat_end", "mud.echo('戰鬥結束')", 10)             -- 之後執行
```

### 一次性監聽

```lua
-- 只觸發一次就自動移除
mud.once("connected", "mud.send('look')")
```

### 觸發事件

```lua
-- 無資料
mud.emit("combat_end")

-- 帶資料（table 會自動序列化為 JSON）
mud.emit("item_found", { name = "寶劍", value = 1000 })
```

### 在 handler 中存取事件資料

handler 執行時，`event_data` 全域變數會被設為事件的 JSON 資料：

```lua
mud.on("item_found", [[
    if event_data then
        mud.echo("收到事件資料: " .. event_data)
        -- event_data 是 JSON 字串，例如: {"name":"寶劍","value":1000}
    end
]])
```

### 取消監聽

```lua
local id = mud.on("test", "mud.echo('test')")
mud.off(id)  -- 取消這個 handler
```

### 內建事件

客戶端會自動觸發以下事件：

| 事件名稱 | 觸發時機 | event_data |
|----------|---------|------------|
| `connected` | 連線成功 | `{"server":"host:port"}` |
| `disconnected` | 斷線 | nil |
| `room_changed` | 偵測到新房間 | `{"id":"hash","name":"房間名"}` |
| `command_sent` | 送出指令 | `{"command":"指令內容"}` |
| `state_changed` | 狀態機轉換 | `{"machine":"名稱","old":"舊狀態","new":"新狀態"}` |

```lua
-- 範例：連線後自動執行
mud.once("connected", "mud.send('set brief'); mud.send('look')")

-- 範例：房間偵測紀錄
mud.on("room_changed", [[
    mud.echo("[Map] 進入: " .. (event_data or "unknown"))
]])
```

---

## 觸發器群組

群組讓你一次開關多個觸發器，適合「模式切換」場景。

### 定義帶群組的觸發器

在 Profile 設定中，觸發器可以指定 `group` 欄位。在 Lua 腳本中，目前需透過 GUI 的觸發器編輯介面或 config JSON 設定 group。

### 批量開關

```lua
-- 禁用整個群組
mud.enable_group("combat", false)

-- 啟用整個群組
mud.enable_group("combat", true)
```

### 典型用法：模式切換

```lua
-- 進入戰鬥模式
function enter_combat_mode()
    mud.enable_group("combat", true)
    mud.enable_group("explore", false)
    mud.echo("[Mode] Combat ON")
end

-- 進入探索模式
function enter_explore_mode()
    mud.enable_group("combat", false)
    mud.enable_group("explore", true)
    mud.echo("[Mode] Explore ON")
end
```

---

## 狀態機

當你的腳本邏輯涉及「在不同狀態下做不同事」時，用狀態機比一堆 if/else flag 清晰得多。

### 建立狀態機

```lua
mud.state_machine("練功bot", {
    -- 初始狀態
    initial = "idle",

    -- 狀態定義
    states = {
        idle = {
            enter = [[
                mud.enable_group("combat", false)
                mud.echo("[Bot] 閒置中")
            ]],
        },
        fighting = {
            enter = [[
                mud.enable_group("combat", true)
                mud.echo("[Bot] 戰鬥中!")
            ]],
            exit = [[
                mud.enable_group("combat", false)
            ]],
            -- 超時保護：60 秒沒結束就強制回 idle
            timeout = { seconds = 60, target = "idle" },
        },
        looting = {
            enter = [[
                mud.send("get all from corpse")
                mud.echo("[Bot] 撿取中...")
            ]],
            timeout = { seconds = 10, target = "idle" },
        },
    },

    -- 轉換規則
    transitions = {
        { from = "idle",     event = "combat_start", to = "fighting" },
        { from = "fighting", event = "combat_end",   to = "looting"  },
        { from = "looting",  event = "loot_done",    to = "idle"     },
    },
})
```

**重要**：`enter` 和 `exit` 是 Lua **程式碼字串**，不是函數。使用 `[[ ]]` 長字串語法可以跨行且不需轉義引號。

### 事件驅動轉換

狀態機會自動監聽事件。當你 `mud.emit("combat_start")` 時，所有狀態機都會檢查是否有匹配的轉換規則。

搭配觸發器使用：

```lua
-- 觸發器偵測到戰鬥開始/結束，發出事件
-- 在 GUI 觸發器設定中：
--   pattern: "starts to fight"
--   action (script): mud.emit("combat_start")
--
--   pattern: "已經死了"
--   action (script): mud.emit("combat_end")
```

### 查詢與手動控制

```lua
-- 查詢目前狀態
local state = mud.sm_current("練功bot")
mud.echo("目前狀態: " .. (state or "未定義"))

-- 手動觸發轉換
mud.sm_transition("練功bot", "combat_start")

-- 重置到初始狀態
mud.sm_reset("練功bot")
```

### timeout（超時保護）

超時是防止狀態卡住的安全機制。例如戰鬥狀態超過 60 秒沒收到 `combat_end`，自動回 idle：

```lua
fighting = {
    timeout = { seconds = 60, target = "idle" },
}
```

超時檢查與 timer 在同一個更新迴圈中進行，精度約 50ms。

---

## 快捷鍵綁定

將鍵盤快捷鍵綁定到 Lua 程式碼。

### 綁定

```lua
-- 功能鍵
mud.bind_key("f5", "mud.send('score')")
mud.bind_key("f9", "mud.sm_reset('練功bot'); mud.echo('Bot 已重置')")

-- Ctrl 組合鍵（macOS 上 Cmd 也算 Ctrl）
mud.bind_key("ctrl+1", "mud.send('drink heal')")
mud.bind_key("ctrl+2", "mud.send('cast fireball')")

-- Alt 組合鍵
mud.bind_key("alt+r", "mud.send('report')")

-- 取消綁定
mud.unbind_key("f5")
```

### 支援的按鍵

- **功能鍵**：`f1` ~ `f12`
- **修飾鍵**：`ctrl`、`alt`、`shift`（可組合，如 `ctrl+shift+a`）
- **字母**：`a` ~ `z`
- **數字**：`0` ~ `9`
- **特殊**：`space`、`tab`、`escape`

格式一律小寫，用 `+` 連接，例如 `ctrl+shift+f5`。

### 注意事項

- 自訂綁定**優先於**內建快捷鍵（F2=設定、F3=Profile）
- 如果你綁定了 `f2`，它會覆蓋設定面板的開關
- 綁定是 session 級別的，不會持久化。通常在啟動腳本中定義
- macOS 上 Cmd 鍵等同 `ctrl`

---

## 訊息路由

將符合 pattern 的伺服器訊息自動導向子視窗。

### 新增路由規則

```lua
-- 將閒聊頻道導向 "chat" 視窗
mud.add_route({
    name    = "chat",       -- 規則名稱（唯一）
    pattern = "【閒聊】",    -- 正則表達式匹配
    window  = "chat",       -- 目標視窗名稱（自動建立）
    gag     = false,        -- true = 從主視窗隱藏
})

-- 系統公告導向 "system" 視窗，並從主視窗隱藏
mud.add_route({
    name    = "system",
    pattern = "\\[公告\\]",
    window  = "system",
    gag     = true,          -- 主視窗不顯示
})
```

### 移除規則

```lua
mud.remove_route("chat")
```

### 運作方式

- 路由規則在**觸發器之後**檢查
- 匹配的訊息會**複製**到目標視窗（`gag=false`）或**只送到**目標視窗（`gag=true`）
- 目標視窗不存在時自動建立
- `pattern` 支援 Rust regex 語法
- 只對**伺服器訊息**生效，`mud.echo()` 的本地訊息不受影響

### 實用場景

```lua
-- 分離聊天頻道
mud.add_route({ name = "chat", pattern = "【閒聊】|【門派】|告訴你", window = "chat", gag = false })

-- 過濾戰鬥垃圾訊息
mud.add_route({ name = "combat_spam", pattern = "你對.*造成.*點傷害", window = "combat_log", gag = true })

-- 收集掉落物品
mud.add_route({ name = "loot", pattern = "你從.*拿了", window = "loot", gag = false })
```

---

## Debug 面板

側邊欄點選 **Debug** 頁籤，提供：

| 區塊 | 功能 |
|------|------|
| **Lua Console** | 即時輸入 Lua 程式碼並查看返回值 |
| **Variables** | 查看所有 `$var` 持久化變數 |
| **State Machines** | 每個狀態機的名稱和目前狀態 |
| **Key Bindings** | 已綁定的快捷鍵列表 |
| **Route Rules** | 已設定的訊息路由規則 |
| **Event Log** | 最近 20 筆觸發的事件（含時間、名稱、資料） |

Debug 面板是即時的，可以在遊戲過程中隨時查看系統狀態。Lua Console 特別適合測試新的事件/狀態機邏輯。

---

## 實戰範例：自動練功 Bot

以下是一個完整的自動練功腳本，整合了所有新功能：

```lua
-- grinder.lua — 自動練功 Bot
-- 用法：在啟動腳本中 dofile("scripts/grinder.lua")

-- ============================================================
-- 1. 訊息路由：分離戰鬥日誌
-- ============================================================
mud.add_route({
    name = "combat_log",
    pattern = "造成.*傷害|閃避了|你對.*發動",
    window = "combat",
    gag = true,
})

-- ============================================================
-- 2. 觸發器群組：戰鬥技能
-- ============================================================
-- 這些觸發器需要在 GUI 中設定，group 設為 "grind_combat"
-- 例如：
--   pattern: "(hp%d+/%d+ .* mv(%d+)/%d+)"
--   action:  根據 mv 選擇技能
--   group:   grind_combat

-- ============================================================
-- 3. 狀態機
-- ============================================================
mud.state_machine("grinder", {
    initial = "idle",
    states = {
        idle = {
            enter = [[
                mud.enable_group("grind_combat", false)
                mud.echo("[Grinder] 閒置中，等待指令")
            ]],
        },
        hunting = {
            enter = [[
                mud.echo("[Grinder] 尋找目標...")
                mud.send("kill monster")
            ]],
        },
        fighting = {
            enter = [[
                mud.enable_group("grind_combat", true)
                mud.echo("[Grinder] 戰鬥中!")
            ]],
            exit = [[
                mud.enable_group("grind_combat", false)
            ]],
            timeout = { seconds = 90, target = "idle" },
        },
        looting = {
            enter = [[
                mud.send("get all from corpse")
                mud.timer(2, "mud.emit('loot_done')")
            ]],
            timeout = { seconds = 10, target = "idle" },
        },
        resting = {
            enter = [[
                mud.echo("[Grinder] 休息中...")
                mud.send("sleep")
                mud.timer(15, "mud.send('wake'); mud.emit('rested')")
            ]],
        },
    },
    transitions = {
        { from = "idle",     event = "start_grind",  to = "hunting"  },
        { from = "hunting",  event = "combat_start",  to = "fighting" },
        { from = "fighting", event = "combat_end",    to = "looting"  },
        { from = "fighting", event = "low_hp",        to = "resting"  },
        { from = "looting",  event = "loot_done",     to = "hunting"  },
        { from = "resting",  event = "rested",        to = "hunting"  },
    },
})

-- ============================================================
-- 4. 事件偵測觸發器（透過 emit 橋接）
-- ============================================================
-- 以下用內聯方式示範，實際使用中建議在 GUI 設定

-- 偵測戰鬥開始（根據你的 MUD 調整 pattern）
mud.on("command_sent", [[
    local data = event_data or ""
    if data:find('"command":"kill') or data:find('"command":"f ') then
        mud.emit("combat_start")
    end
]])

-- 偵測戰鬥結束
-- 注意：這通常由觸發器做，這裡只是示範
-- trigger: pattern="已經死了" → action: mud.emit("combat_end")

-- ============================================================
-- 5. 狀態變更紀錄
-- ============================================================
mud.on("state_changed", [[
    mud.echo("[Grinder] " .. (event_data or ""))
]], 100)  -- 低優先序，紀錄用

-- ============================================================
-- 6. 快捷鍵控制
-- ============================================================
mud.bind_key("f5", [[
    mud.emit("start_grind")
    mud.echo("[Grinder] 開始練功!")
]])

mud.bind_key("f6", [[
    mud.sm_reset("grinder")
    mud.echo("[Grinder] 已停止")
]])

mud.bind_key("f7", [[
    local state = mud.sm_current("grinder") or "未啟動"
    mud.echo("[Grinder] 狀態: " .. state)
]])

mud.echo("[Grinder] 已載入! F5=開始 F6=停止 F7=狀態")
```

### 運作流程

```
按 F5 → emit("start_grind")
  → idle → hunting (send "kill monster")
    → 觸發器偵測到戰鬥 → emit("combat_start")
      → hunting → fighting (enable combat triggers)
        → 觸發器偵測到敵人死亡 → emit("combat_end")
          → fighting → looting (get all, 2秒後 emit loot_done)
            → looting → hunting (再次 kill)
              → 循環...

按 F6 → sm_reset → 回到 idle，combat triggers 關閉
按 F7 → 顯示目前狀態
```

---

## 常見問題

### Q: 同一個 Lua 呼叫中可以同時定義 state_machine 又 emit 嗎？

可以。`mud.state_machine()` 的定義會在 `mud.emit()` 之前處理，所以在同一個腳本中定義完馬上 emit 是有效的。

### Q: 事件 handler 中可以再 emit 嗎？

可以，但注意避免無限迴圈。例如 `state_changed` 事件的 handler 中不要再觸發會導致狀態轉換的事件。

### Q: 快捷鍵綁定會保存嗎？

不會。快捷鍵是 runtime 設定，通常在啟動腳本（`scripts/` 目錄下）中定義，每次啟動自動載入。

### Q: 路由規則對 mud.echo 有效嗎？

無效。路由規則只匹配伺服器發來的訊息，不匹配本地 echo。這是設計如此。

### Q: 狀態機超時的精度是多少？

約 50ms（與 GUI 刷新頻率相同）。對 MUD 遊玩場景足夠精確。

### Q: enter/exit callback 中可以用多行程式碼嗎？

可以。使用 Lua 長字串 `[[ ]]` 或 `[=[ ]=]` 語法：

```lua
fighting = {
    enter = [[
        mud.enable_group("combat", true)
        mud.send("agg 100")
        mud.echo("戰鬥模式!")
    ]],
}
```

### Q: 怎麼在 handler 中解析 event_data？

`event_data` 是 JSON 字串。如果需要欄位值，可以用 string.match：

```lua
mud.on("room_changed", [[
    local name = (event_data or ""):match('"name":"(.-)"')
    if name then
        mud.echo("進入: " .. name)
    end
]])
```

或者載入 JSON 解析庫（如果有的話）。

---

## 相關文件

- [指令與腳本指南 (API Reference)](API.md) — 完整的 API 函數清單
- [腳本開發指南 (Script Guide)](ScriptGuide.md) — 腳本架構與模組系統
- [觸發器與別名指南](Triggers_Aliases.md) — 觸發器和別名設定
