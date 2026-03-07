# QuestEngine 教學指南

> **適用版本**：mudclient-rs v0.9.0+
> **最後更新**：2026-03-07

QuestEngine 是一個宣告式任務引擎，讓你用 ~30 行資料定義取代 600+ 行的命令式任務腳本。它在底層協調 MudNav、MudExplorer、MudCombat、MudLoot、MudUtils 等模組。

---

## 目錄

1. [快速開始](#快速開始)
2. [任務定義格式](#任務定義格式)
3. [Step 類型參考](#step-類型參考)
4. [路徑格式](#路徑格式)
5. [進階功能](#進階功能)
6. [撰寫新任務的流程](#撰寫新任務的流程)
7. [實戰範例](#實戰範例)
8. [API 參考](#api-參考)
9. [常見問題](#常見問題)

---

## 快速開始

### 載入與執行

```lua
-- 載入任務腳本（自動載入 QuestEngine 模組）
/lua dofile("scripts/poker_quest_v2.lua")

-- 執行
/lua PokerQuestV2.start()

-- 停止
/lua PokerQuestV2.stop()

-- 查看狀態
/lua PokerQuestV2.status()
```

### 最簡任務

```lua
local QuestEngine = require("scripts.modules.QuestEngine")

QuestEngine.define("hello_quest", {
    steps = {
        {type="navigate", name="go_south", path="3s"},
        {type="say", name="greet", text="hello", expect="你好"},
    }
})

_G.HelloQuest = {
    start = function() QuestEngine.run("hello_quest") end,
    stop = function() QuestEngine.stop() end,
}
```

這個任務會：往南走 3 步 → 說 hello → 等伺服器回應包含「你好」→ 完成。

---

## 任務定義格式

```lua
QuestEngine.define("quest_name", {
    -- 任務級選項（皆為可選）
    recall_cmd = "recall",        -- 執行前先 recall
    log_name = "quest_log",       -- 啟用日誌記錄
    loop = true,                  -- 完成後自動重跑（間隔 10 秒）
    precheck = {"npc_id1"},       -- 啟動前檢查 NPC 是否存在
    on_fail = {                   -- 失敗時的清理動作
        path = {"n", "n"},
        cmds = {"say sorry"},
    },

    -- 步驟定義（依序執行）
    steps = {
        {type="navigate", name="step1", ...},
        {type="hunt",     name="step2", ...},
        {type="say",      name="step3", ...},
        -- ...
    }
})
```

### 任務級選項

| 選項 | 類型 | 說明 |
|------|------|------|
| `recall_cmd` | string | 啟動任務前發送的回城指令 |
| `log_name` | string | 日誌檔名（存入 `logs/` 目錄） |
| `loop` | boolean | 完成後自動重新執行（10 秒間隔） |
| `precheck` | table | NPC ID 列表，啟動前用 `q <id>` 檢查存在性 |
| `on_fail` | table | 失敗時清理：`path`（回城路徑）+ `cmds`（清理指令） |

---

## Step 類型參考

### navigate — 路徑導航

沿指定路徑移動到目的地。

```lua
{type="navigate", name="go_to_town",
 path = "3s;2e;u",              -- 路徑（字串或 table）
 expect = "城門口",              -- 可選：到達後等待匹配文字
 cmds = {"look", "bow guard"},  -- 可選：到達後執行指令
 on_fail = "retry",             -- 可選："retry" 或 "stop"（預設）
 timeout = 30.0,                -- 可選：expect 超時秒數（預設 30）
}
```

### hunt — 探索並獵殺目標

使用 MudExplorer 搜索區域，找到目標後戰鬥、撿取戰利品，完成後回溯到起點。

```lua
{type="hunt", name="find_boss",
 target = "goblin",                -- 擊殺偵測用的關鍵字（不分大小寫）
 explorer_target = "(goblin)正站在", -- 可選：MudExplorer 搜尋用的完整匹配字串
 attack_cmd = "kill goblin",       -- 攻擊指令
 max_laps = 5,                     -- 可選：探索最大圈數（預設 5）
 disable_open_doors = true,        -- 可選：禁用自動開門（預設 false）
 debug = false,                    -- 可選：MudExplorer debug 模式
 loot = {                          -- 可選：戰利品設定
     items = {"stone", "key"},     -- 需要的物品關鍵字
     sac = true,                   -- 是否 sacrifice 屍體
     loot_ground = true,           -- 是否撿地上物品
     fallback_blind = true,        -- 盲撿模式
 },
}
```

**流程**：探索 → 找到目標 → 戰鬥 → 撿取 → 拿到指定物品？ → 是：回溯起點 → 進入下一步 / 否：繼續探索

**`target` vs `explorer_target`**：
- `target`：用於擊殺偵測（`xxx魂歸西天了`），不分大小寫匹配
- `explorer_target`：用於 MudExplorer 的房間掃描匹配，如 `(spade)正站在` 可避免匹配屍體

### give — 交付物品

```lua
{type="give", name="give_key",
 item = "key",            -- 物品關鍵字
 npc = "king",            -- NPC 關鍵字
 expect = "國王收下了",    -- 可選：等待確認文字
 timeout = 30.0,          -- 可選：超時秒數
}
```

發送 `gi <item> <npc>` 指令。

### say — 對話

```lua
{type="say", name="greet_queen",
 text = "say goodmorning",       -- 對話指令（自動加 "say" 前綴）
 expect = "王后說道: 歡迎",       -- 可選：等待回應
 timeout = 30.0,                 -- 可選：超時秒數
}
```

如果 `text` 不以 `say`/`ta`/`talk` 開頭，會自動加上 `say` 前綴。

### interact — 通用互動

最靈活的 step 類型，可發送任意指令。

```lua
{type="interact", name="push_stone",
 cmd = "push stone",            -- 指令（字串或 table）
 expect = "石頭移開了",          -- 可選：等待回應
 retry = 3,                     -- 可選：超時後重試次數
 timeout = 10.0,                -- 可選：超時秒數（預設 30）
}

-- 多指令版本
{type="interact", name="complex_action",
 cmd = {"look", "gi stone king", "bow king"},
 expect = "國王點了點頭",
}
```

### summon — 召喚 NPC

使用 MudCombat.safe_summon 召喚目標，支援重試和自動偵測。

```lua
{type="summon", name="summon_npc",
 target = "老人",                   -- 目標名稱（用於偵測）
 summon_cmd = "cast 'summon' old",  -- 召喚指令
 max_retries = 5,                   -- 可選：最大重試次數（預設 5）
 retry_delay = 3.0,                 -- 可選：重試間隔秒數
 verify_delay = 2.0,                -- 可選：驗證延遲秒數
 path = {"n", "e"},                 -- 可選：先移動到指定位置
 cmds = {"give item old"},          -- 可選：召喚成功後執行指令
 expect = "老人收下了",              -- 可選：等待回應
}
```

**智能偵測**：會先用 `l` 檢查目標是否已在房間中，避免浪費召喚。

### wait_for_mob — 等待 NPC 出現

在指定位置輪詢等待目標 NPC 出現。適合 NPC 會隨機移動的場景。

```lua
{type="wait_for_mob", name="wait_npc",
 target = "cat",                   -- 目標關鍵字（匹配 "(cat)"）
 target_alias = "小花貓",          -- 可選：中文名稱匹配
 path = {"n", "n"},               -- 可選：先移動到等待位置
 summon_cmd = "cast 'summon' cat", -- 可選：每 10 秒嘗試召喚
 cmds = {"pet cat"},              -- 可選：找到後執行指令
 expect = "小花貓喵了一聲",        -- 可選：等待回應
 wait_timeout = 60.0,             -- 可選：等待超時秒數
 timeout_cmds = {"say 算了"},     -- 可選：超時時執行指令
 timeout_skip = 2,                -- 可選：超時時跳過 N 步
}
```

**輪詢機制**：每 5 秒發送 `l` 檢查目標，如設定 `summon_cmd` 則每 10 秒嘗試召喚。

### custom — 自訂邏輯

當內建 step 類型不夠用時，寫 inline 函數。

```lua
{type="custom", name="special_logic",
 expect = "特殊文字",        -- 可選：設定 expect 等待
 timeout = 30.0,            -- 可選：expect 超時
 fn = function(step, engine)
     -- 你的自訂邏輯
     mud.send("special command")
     -- 如果沒設 expect，需手動呼叫 QuestEngine.advance()
 end,
}
```

---

## 路徑格式

navigate 和其他支援 `path` 的 step 接受三種格式：

### 字串格式（簡潔）

```lua
path = "3s;2e;u;u"    -- 3步南、2步東、2步上
path = "n;e;s;w"      -- 北東南西
```

### 陣列格式（明確）

```lua
path = {"s", "s", "s", "e", "e", "u", "u"}
```

### 混合格式（精確控制）

```lua
path = {
    "s", "s", "w",                            -- 普通方向
    {cmd="u", id="abc123..."},                 -- Room ID 驗證步
    {action="push stone", expect="移開了"},     -- 條件動作
}
```

- **`{cmd, id}`**：發送 `cmd` 後，驗證 `mud.get_current_room_id()` 是否匹配
- **`{action, expect, cond}`**：先檢查 `cond`（可選），執行 `action`，等待 `expect` 回應

---

## 進階功能

### Precheck（NPC 存在性檢查）

啟動任務前自動檢查 NPC 是否在線。不存在時每 30 秒重試。

```lua
QuestEngine.define("quest", {
    precheck = {"npc_keyword1", "npc_keyword2"},
    steps = { ... }
})
```

### Loop（自動循環）

任務完成後自動重新執行，適合刷怪腳本。

```lua
QuestEngine.define("grind_quest", {
    loop = true,          -- 完成後 10 秒重新開始
    recall_cmd = "recall",
    steps = { ... }
})
```

### on_fail（失敗清理）

任務失敗時自動回城並執行清理指令。

```lua
QuestEngine.define("quest", {
    on_fail = {
        path = {"n", "n", "w"},    -- 失敗後走的路徑
        cmds = {"drop all"},       -- 清理指令
    },
    steps = { ... }
})
```

### 成功自動回城

設定 `recall_cmd` 後，任務成功完成時會自動 recall 回城，讓 NPC 有時間重生。

---

## 撰寫新任務的流程

### 步驟 1：手動跑一次任務

在遊戲中手動完成任務一次，記錄：
- 每一步的移動路徑
- 需要對話的 NPC 和關鍵對白
- 需要擊殺的怪物
- 需要拾取和交付的物品
- 關鍵房間的 Room ID（在 Debug 面板或用 `mud.get_room_id()` 取得）

### 步驟 2：定義步驟

將任務拆解為一連串的 step：

```lua
-- 大部分任務的通用結構：
-- 1. navigate — 走到任務起點
-- 2. interact/say — 接受任務
-- 3. navigate — 走到目標區域
-- 4. hunt — 找怪殺怪拿物品
-- 5. navigate — 走回交付點
-- 6. give/interact — 交付物品
-- 7. say — 完成對話
```

### 步驟 3：設定 expect

每個需要確認的步驟都加上 `expect`，確保引擎等到伺服器回應後才進入下一步。選擇**穩定且唯一**的文字片段作為 expect。

### 步驟 4：測試

```lua
/lua dofile("scripts/my_quest.lua")
/lua MyQuest.start()
```

觀察 `[QuestEngine]` 的 echo 輸出來追蹤進度。如果卡住，用 `MyQuest.stop()` 停止。

### 步驟 5：讓 AI 幫你寫

你也可以將手動跑任務的遊戲記錄提供給 Claude，讓 AI 自動生成 QuestEngine 格式的腳本。這是 QuestEngine 設計的核心目標。

---

## 實戰範例

### 範例 1：撲克王國任務（poker_quest_v2.lua）

```lua
local QuestEngine = require("scripts.modules.QuestEngine")

QuestEngine.define("poker_quest", {
    recall_cmd = "recall",
    log_name = "poker",
    steps = {
        -- 1. 走到凍原山頂
        {type="navigate", name="go_mountain_top",
         path="6s;2e;4u"},

        -- 2. 等待傳送到撲克王國
        {type="custom", name="enter_poker_kingdom",
         expect="紙路",
         fn=function(step, engine)
             -- 凍原山頂會自動 sleep → 傳送
         end},

        -- 3. 獵殺黑桃小兵取得黃色石頭
        {type="hunt", name="hunt_spade",
         target="spade",
         explorer_target="(spade)正站在",
         attack_cmd="ear spade",
         max_laps=5,
         disable_open_doors=true,
         loot={items={"stone"}, sac=true}},

        -- 4. 走到方塊國王處
        {type="navigate", name="deliver_to_king",
         path={"n","n","w","w","n","n",
               {cmd="w", id="6faf86d9..."}}},

        -- 5. 交付石頭
        {type="interact", name="give_stone",
         cmd="gi stone king", expect="給了"},

        -- 6. 走到黑桃王后處
        {type="navigate", name="go_to_queen",
         path={"e","n","n","e","e",
               {cmd="n", id="68e309f3..."}}},

        -- 7. 問候王后取得密語
        {type="say", name="talk_queen",
         text="say goodmorning",
         expect="黑桃王后說道"},

        -- 8. 走到紅心女王處說密語
        {type="navigate", name="go_to_palace",
         path={"s","s","s","u","u",
               {cmd="u", id="fd4a9da7..."}}},

        {type="say", name="say_password",
         text="say ireallywantleave",
         expect="紅心女王說道"},
    }
})

_G.PokerQuestV2 = {
    start = function() QuestEngine.run("poker_quest") end,
    stop = function() QuestEngine.stop() end,
}
```

### 範例 2：帶 NPC 等待和循環的任務

```lua
QuestEngine.define("npc_quest", {
    recall_cmd = "recall",
    loop = true,            -- 完成後自動重跑
    precheck = {"npc_id"},  -- 檢查 NPC 是否在線
    on_fail = {path = "recall"},
    steps = {
        {type="navigate", name="go_to_area", path="4n;2w"},

        -- 等待會隨機走動的 NPC
        {type="wait_for_mob", name="wait_npc",
         target="cat", target_alias="小花貓(Cat)",
         summon_cmd="cast 'summon' cat",
         cmds={"give fish cat"},
         expect="小花貓高興地收下了"},

        {type="navigate", name="go_back", path="2e;4s"},

        {type="interact", name="report",
         cmd="say done", expect="任務完成"},
    }
})
```

---

## API 參考

### QuestEngine.define(name, def)

定義一個任務。

- `name` (string) — 任務名稱（唯一識別）
- `def` (table) — 任務定義，包含 `steps` 陣列和可選的任務級選項

### QuestEngine.run(name)

啟動指定任務。如果有任務正在執行，會先停止舊任務。

### QuestEngine.stop([is_success])

停止當前任務。

- `is_success` (boolean, 可選) — `true` 表示成功完成，`false` 或省略表示中止

### QuestEngine.advance()

手動推進到下一步。通常由引擎自動呼叫，但在 `custom` step 中可能需要手動呼叫。

### QuestEngine.set_expect(expect, timeout, label)

設定 expect 等待。通常由引擎自動處理，但在 `custom` step 中可使用。

- `expect` (string) — 等待匹配的文字
- `timeout` (number, 可選) — 超時秒數（預設 30）
- `label` (string, 可選) — 超時訊息標籤

### QuestEngine.state

當前執行狀態（table），包含：

| 欄位 | 類型 | 說明 |
|------|------|------|
| `running` | boolean | 是否正在執行 |
| `quest_name` | string | 當前任務名稱 |
| `step_index` | number | 當前步驟索引（從 1 開始） |
| `run_id` | number | 執行 ID（防止舊 timer 干擾） |
| `phase` | string | 當前階段（navigating, finding, fighting 等） |
| `hunt_kills` | number | 獵殺步驟的擊殺計數 |
| `hunt_got_item` | boolean | 是否已拾取目標物品 |

---

## 常見問題

### Q: 任務卡在某一步怎麼辦？

先用 `QuestEngine.stop()` 停止，然後檢查：
1. `expect` 是否正確？用固定文字匹配（plain text），確保伺服器回應中包含該字串
2. 路徑是否正確？可以手動走一次確認
3. 查看 `[QuestEngine]` 的 echo 輸出判斷卡在哪個階段

### Q: `target` 和 `explorer_target` 有什麼區別？

- `target`：用於擊殺偵測，匹配「xxx魂歸西天了」中的 xxx
- `explorer_target`：用於 MudExplorer 搜尋房間時的匹配文字。設定為 `(spade)正站在` 等格式可以避免匹配到屍體行

如果不設定 `explorer_target`，會直接使用 `target`。

### Q: 如何處理需要特殊邏輯的步驟？

使用 `custom` step 類型，寫 inline 函數。如果需要等待伺服器回應，用 `expect` 或手動呼叫 `QuestEngine.set_expect()`。完成後呼叫 `QuestEngine.advance()`。

### Q: 怎麼在任務腳本中加入自訂的 hook？

用 `MudUtils.register_hook()` 註冊額外的訊息處理，在 hook 中檢查 `QuestEngine.state.running` 和 `quest_name` 確保只在對的任務中觸發。參見 `poker_quest_v2.lua` 的做法。

### Q: loop 模式下如何停止？

直接呼叫 `QuestEngine.stop()` 即可。loop 只在任務**成功完成**時才會重新啟動。

### Q: precheck 失敗會怎樣？

會每 30 秒重試檢查。任務會保持在 `prechecking` 階段直到所有 NPC 都存在，或手動 `stop()`。

---

## 相關文件

- [腳本開發指南 (Script Guide)](ScriptGuide.md) — 腳本架構與模組系統
- [事件驅動系統教學 (Event System)](EventSystem.md) — 事件、狀態機、快捷鍵
- [導航與地圖系統 (Navigation)](Navigation.md) — MudNav、MudExplorer 詳細說明
- [指令與腳本 API (API Reference)](API.md) — 完整的 mud.* API 清單
