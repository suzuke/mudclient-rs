# MCP Agent Playbook — 讓 AI 遊玩 MUD 的完整指南

本文件詳細說明如何讓 AI Agent（如 Claude / Gemini / GPT）透過 MCP Server 操控 `mudclient-rs`，在文字 MUD 遊戲中進行偵查、導航、戰鬥與任務自動化。

> [!NOTE]
> **多 Session 支援**：MUD 客戶端支援同時開啟多個 Session（分身），每個 Session 擁有獨立的訊息緩衝區和指令佇列。所有 API endpoint 和 MCP Tool 支援可選的 `session` 參數，不指定時預設使用當前 active session。可透過 `list_sessions` 查看所有 Session 的 key。

---

## 一、可用工具總覽

| MCP Tool | 用途 | 回應方式 |
|----------|------|----------|
| `get_status` | 取得連線狀態與 Session 名稱 | 即時 JSON |
| `get_room_info` | 取得當前房間名、描述、出口、Room ID | 即時 JSON |
| `read_messages` | 讀取最近 N 行遊戲訊息（含戰鬥、對話、系統訊息） | 即時 JSON |
| `send_command` | 發送遊戲指令（移動、攻擊、施法、對話等） | Fire-and-forget |
| `execute_lua` | 執行 Lua 腳本，無回傳值（用於觸發操作） | Fire-and-forget |
| `evaluate_lua` | 執行 Lua 腳本並**等待回傳結果**（用於查詢狀態） | 同步回傳 JSON |
| `clear_messages` | 清空訊息緩衝區 | 即時 JSON |
| `list_sessions` | 列出所有 Session 的 key、名稱、狀態 | 即時 JSON |

> 🔑 所有工具（除 `list_sessions`）都支援可選 `session` 參數。不指定時預設操作當前 active session。

---

## 二、遊戲世界基礎知識

### 2.1 狀態列 (Prompt)

每次行動後，伺服器會回傳一行狀態列：

```
(hp2741/2741 ma1984/2154 v1145/1364 p311/311 64543 善良)
```

| 欄位 | 含義 | 說明 |
|------|------|------|
| `hp2741/2741` | 生命力 (HP) | 當前/最大 |
| `ma1984/2154` | 精神力 (MA/MP) | 施法消耗，用完就無法施法 |
| `v1145/1364` | 移動力 (V/MV) | 每次移動消耗，用完無法移動 |
| `p311/311` | 內力 (P) | 特殊技能消耗 |
| `64543` | 經驗值 (EXP) | — |
| `善良` | 陣營 | 善良/中立/邪惡 |

> **⚠️ 重要**：Agent 可從 `read_messages` 解析此狀態列來判斷角色健康狀態。低 HP 應考慮逃跑或補血。

### 2.2 房間描述

房間描述的標準格式：

```
市中心
這裡正是風采之都的市中心, 繁華的街道...
(ID: 4f918fc15a8b069659b31992056d461a0cfa7d5f582328ecf6155c1ba7124796)
[出口: 北 東 南 西]
一台老舊而運作聲超大的飲水機/spring
(bala_ny14_geo_checking_mob) 站在這兒.
```

| 區段 | 內容 |
|------|------|
| 第一行 | 房間名稱 |
| 描述段 | 環境描述 |
| `(ID: ...)` | 房間唯一 ID（由客戶端計算） |
| `[出口: ...]` | 可用方向（北/南/東/西/上/下） |
| 物品/NPC 行 | `名稱/英文id` 格式，或 `某NPC 站在這兒.` |

> **💡 提示**：也可以直接用 `get_room_info` 取得結構化房間資訊，不需要自己解析文字。

### 2.3 戰鬥系統

戰鬥為**自動回合制**。發起攻擊後（`kill <target>`），系統自動進行普通攻擊，玩家可額外輸入特殊技能指令。

#### 戰鬥訊息範例

```
你拿起手中的冶鍊超Ｚ合金短刀向 賈不妙 的左臂全力一戳, 使 賈不妙 不得不按住自己的傷口.(75)
 [x2]
賈不妙 已經面無血色了. 你心裡正盤算著對手的下一招..[332]
```

| 元素 | 含義 |
|------|------|
| `(75)` | 本次攻擊造成的傷害值 |
| `[x2]` | 連擊次數 |
| `已經面無血色了` | 對手的**文字狀態描述**（見下表） |
| `..[332]` | **協調性 (Coordination)**，不是對手 HP |

#### 對手狀態描述（由健康到瀕死）

| 文字描述 | 推測狀態 |
|----------|----------|
| 你心裡正盤算著對手的下一招 | 仍有戰鬥力 |
| 你正蓄勢待發 | 對手虛弱 |
| XXX 不停地在流血 | 對手生命持續流失 |
| XXX 已經面無血色了 | 對手瀕死 |
| XXX 魂歸西天了!! | **對手死亡** ✅ |

> **⚠️ 關鍵**：MUD 遊戲**無法直接得知對手的精確血量**。只能透過上述文字描述推測。

#### 協調性 (Coordination)

方括號中的數字 `[332]` 是**協調性**，代表角色的攻擊節奏指標：
- 每次攻擊會增加協調性
- **超過 1000 無法繼續攻擊**，必須等待降低
- 不同攻擊方式消耗不同的協調性

#### 戰鬥結束標誌

- 成功擊殺：`XXX 魂歸西天了!!`
- 逃跑成功：`你為了保命而不顧面子從戰鬥中逃了!`
- 對方逃跑：`XXX 為了保命而不顧面子從戰鬥中逃了!`

### 2.4 常用指令速查

| 指令 | 說明 |
|------|------|
| `look` / `l` | 查看當前房間 |
| `n` / `s` / `e` / `w` / `u` / `d` | 移動（北/南/東/西/上/下） |
| `recall` | 傳送回城（Recall 點） |
| `score` | 查看角色詳細資訊 |
| `i` | 查看背包 |
| `kill <target>` | 發動攻擊 |
| `flee` | 逃離戰鬥 |
| `c <spell> [target]` | 施放法術（如 `c heal`, `c flame boy`） |
| `c sum <target>` | 召喚術（將遠處 NPC 拉到身邊） |
| `c ref` | 恢復移動力 |
| `c sa` | 施放聖光（Buff） |
| `get <item> [from corpse]` | 撿取物品 |
| `sleep` | 休息（加速 HP/MP 恢復）|
| `wa` / `wake` | 起身 |
| `q <mob_id>` | 查詢 NPC 是否存在 |
| `ta <npc> <message>` | 與 NPC 對話 |
| `gi <item> <npc>` | 給予 NPC 物品 |
| `help` | 查看遊戲內建說明系統（入門首選）|
| `help <topic>` | 查看特定主題說明（如 `help combat`）|

> **💡 自學技巧**：Agent 遇到不熟悉的遊戲機制時，可以直接發送 `help` 或 `help <關鍵字>` 來取得遊戲內建的說明文檔，再透過 `read_messages` 讀取內容來學習。這是最可靠的遊戲知識來源。

---

## 三、Agent 操作模式

### 3.1 感知-決策-行動循環 (Perception-Decision-Action Loop)

Agent 與 MUD 互動的核心模式：

```
┌─────────────┐
│  1. 感知     │ ← get_room_info / read_messages / evaluate_lua
│  (Perceive)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  2. 決策     │ ← Agent 的 LLM 推理
│  (Decide)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  3. 行動     │ ← send_command / execute_lua
│  (Act)       │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  4. 等待     │ ← 等待 1-3 秒讓伺服器回應
│  (Wait)      │
└──────┬──────┘
       │
       └──────→ 回到 1
```

### 3.2 精準環境掃描模式

這是最推薦的偵查流程，可避免歷史訊息干擾：

```
Step 1: clear_messages          → 清空緩衝區
Step 2: send_command("look")    → 觸發伺服器輸出
Step 3: (等待 1-2 秒)
Step 4: read_messages(count=20) → 取得乾淨的房間資訊
```

> 💡 或者直接用 `get_room_info`，它會回傳結構化 JSON，不需要解析文字。

### 3.3 導航模式

#### 簡單導航（用分號串接）

```
send_command("recall")          → 回城
send_command("w;w;w;s;s;s;e")   → 一次發送多步路徑
```

#### 確認到達

```
send_command("w;w;w")
(等待 2 秒)
get_room_info()                 → 確認已到達目標房間
```

> **⚠️ 注意**：移動會消耗 V（移動力），不足時會提示「你的移動力不足」。此時需 `c ref` 恢復或 `sleep` 休息。

### 3.4 戰鬥模式

#### 完整戰鬥流程

```
Step 1: send_command("kill <target>")    → 發動攻擊
Step 2: (等待 1 秒)
Step 3: read_messages(count=20)          → 確認進入戰鬥
Step 4: send_command("c flame <target>") → 施放攻擊技能（視角色職業而定）
Step 5: (迴圈) read_messages 直到看見「魂歸西天」
Step 6: send_command("get all corpse")   → 搜刮屍體
```

#### 戰鬥中的重要判斷

| 觀察到的訊息 | 建議動作 |
|-------------|---------|
| HP 低於 50% | `flee` 逃跑或 `c heal` 治療 |
| MA 接近 0 | 停止施法，依靠普攻 |
| 「精疲力竭」 | `c ref` 恢復移動力 |
| 「魂歸西天」 | 戰鬥勝利，進入撿取階段 |
| 「你想攻擊的對象不在這裡」 | 目標已離開或死亡 |

### 3.5 利用現有 Lua 模組

客戶端內建了成熟的 Lua 模組，Agent 可以透過 `evaluate_lua` 直接呼叫：

```lua
-- 查詢 NPC 是否存活（回傳 true/false）
return type(MudCombat) == 'table'

-- 取得當前房間 ID
return mud.get_current_room_id()

-- 檢查是否在戰鬥中
return MudCombat and MudCombat.is_fighting() or false
```

也可以用 `execute_lua` 觸發現有腳本：

```lua
-- 啟動藍色小精靈任務
SmurfQuest.start()

-- 啟動自動掛機打怪
ItemFarm.start()

-- 停止所有腳本
SmurfQuest.stop()
ItemFarm.stop()
```

---

## 四、任務自動化範例

以「藍色小精靈任務 (SmurfQuest)」為例，展示典型的任務流程：

```
1. 預檢：q papa / q gargamel → 確認 NPC 存在
2. 導航：recall → 走到精靈村入口 → 進入村莊
3. 召喚：c sum papa → 召喚老爸
4. 對話：ta papa yes → 取得鑰匙
5. 導航：走到賈不妙城堡 → un n; op n → 解鎖進入
6. 戰鬥：kill gargamel + ear gargamel → 擊殺
7. 搜刮：get wand corpse → 取得魔杖
8. 繳交：c sum papa → gi wand papa → 任務完成
```

每一步都可以映射為 MCP 工具呼叫序列。

---

## 五、常見陷阱與注意事項

### 5.1 多 Session 操作

客戶端支援同時連線多個角色（Session）。每個 Session 擁有獨立的訊息緩衝區和指令佇列。

使用步驟：
1. 先呼叫 `list_sessions` 取得每個 Session 的 `session_key`
2. 將 `session_key` 傳入都其他工具的 `session` 參數
3. 不指定時，預設操作當前 active session（GUI 中正在查看的分頁）

### 5.2 Room ID 需要 `look` 觸發

剛連線或傳送後，`get_room_info` / `evaluate_lua("return mud.get_current_room_id()")` 可能回傳 `null`。這是因為客戶端尚未收到標準格式的房間描述。解決方法：先發送 `look`，等待 1 秒後再查詢。

### 5.3 移動力耗盡

移動力 (V) 不足時移動會失敗。長距離導航前建議先用 `c ref` 補充。如果完全耗盡需要 `sleep` 休息。

### 5.4 法術需要精神力 (MA)

施法消耗 MA。MA 不足時施法會失敗，出現「你精神力不夠!」。需 `sleep` 恢復。

### 5.5 戰鬥中不要移動

戰鬥中無法正常移動（除非 `flee`）。嘗試移動會得到「你正在戰鬥中!」。

### 5.6 協調性上限

當協調性 `[xxx]` 超過 1000 時，角色無法繼續攻擊，只能等待降低。

---

## 六、推薦的 Agent Prompt 模板

以下是建議給 AI Agent 使用的系統提示片段：

```
你是一個 MUD 遊戲助手。你可以透過 MCP 工具控制 MUD 客戶端。

操作原則：
1. 每次行動前先用 clear_messages 清空，再 send_command，再 read_messages
2. 移動前確認移動力 (V) 足夠，不足先 c ref
3. 戰鬥中持續 read_messages 監控，直到看到「魂歸西天」或需要逃跑
4. MUD 無法直接看對手 HP，只能從文字描述推測
5. 方括號中的數字是「協調性」不是血量，超過 1000 無法攻擊
6. 不要盲目猜測遊戲指令，先查閱已有的 Lua 腳本或詢問用戶
```
