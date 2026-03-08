---@meta

-- mudclient-rs Lua API 型別定義
-- 此檔案僅供 LuaLS 使用，不會被執行

---@class mud
---@field commands string[] 待發送的指令佇列（內部使用）
---@field variables table<string, string> 持久化變數（跨腳本共享，自動儲存）
---@field is_echo boolean 當前訊息是否為本地回顯
---@field gag boolean 是否抑制當前訊息顯示
mud = {}

-- ============================================================
-- 基本 I/O
-- ============================================================

--- 送出指令到 MUD 伺服器
---@param cmd string 要送出的指令
function mud.send(cmd) end

--- 抑制當前觸發的訊息，不顯示在畫面上
function mud.gag_message() end

--- 本地顯示訊息（支援 ANSI 色碼如 `{r` 紅色 `{x` 重置）
---@param text string 要顯示的文字
function mud.echo(text) end

--- 輸出訊息到指定子視窗
---@param name string 視窗名稱
---@param text string 要顯示的文字
function mud.window(name, text) end

--- 寫入日誌訊息
---@param msg string 日誌內容
function mud.log(msg) end

--- 開始日誌記錄到檔案
---@param path string 日誌檔案路徑
function mud.start_log(path) end

--- 停止日誌記錄
function mud.stop_log() end

-- ============================================================
-- 計時器
-- ============================================================

--- 延遲執行 Lua 程式碼或函數
---
--- 範例：
--- ```lua
--- -- 字串模式（向下相容）
--- mud.timer(5, "mud.send('look')")
---
--- -- function 模式（推薦：有 closure、IDE 補全、語法檢查）
--- mud.timer(5, function()
---     mud.send("look")
---     mud.echo("已自動 look")
--- end)
--- ```
---@param seconds number 延遲秒數（支援小數，如 0.5）
---@param callback string|function 到期後執行的 Lua 程式碼字串或函數
function mud.timer(seconds, callback) end

-- ============================================================
-- 觸發器控制
-- ============================================================

--- 啟用或禁用指定觸發器
---@param name string 觸發器名稱
---@param enabled boolean true=啟用, false=禁用
function mud.enable_trigger(name, enabled) end

--- 啟用或禁用整個觸發器群組
---@param group_name string 群組名稱
---@param enabled boolean true=啟用, false=禁用
function mud.enable_group(group_name, enabled) end

-- ============================================================
-- 非同步回應收集
-- ============================================================

--- 送出指令並收集伺服器回應（直到下一個 prompt）
---
--- 範例：
--- ```lua
--- -- 字串模式（向下相容，透過 _G._collected_lines 取得回應行）
--- mud.collect_response("inventory", [[
---     for _, line in ipairs(_G._collected_lines) do
---         mud.echo(line)
---     end
--- ]])
---
--- -- function 模式（推薦：lines 直接作為參數傳入）
--- mud.collect_response("inventory", function(lines)
---     for _, line in ipairs(lines) do
---         mud.echo(line)
---     end
--- end)
--- ```
---@param cmd string 要送出的指令
---@param callback string|fun(lines: string[]) 回應收集完畢後執行的 Lua 程式碼或函數
function mud.collect_response(cmd, callback) end

--- 依序送出多個指令，每個等待回應後才發下一個
---
--- 只有最後一個指令的回應會觸發 callback。
---
--- 範例：
--- ```lua
--- mud.send_chain({"cast 'armor'", "cast 'shield'"}, function(lines)
---     mud.echo("施法完成")
--- end)
--- ```
---@param cmds string[] 指令列表
---@param callback? string|fun(lines: string[]) 最後一個指令回應完畢後的 callback（可選）
function mud.send_chain(cmds, callback) end

--- 非同步呼叫 LLM（Claude API）
---
--- 需要設定環境變數 `ANTHROPIC_API_KEY`。
---
--- 範例：
--- ```lua
--- -- 字串模式（$RESULT 被替換為回覆字串）
--- mud.ask_llm("翻譯：hello world", [[
---     mud.echo("翻譯結果: " .. $RESULT)
--- ]])
---
--- -- function 模式（推薦：result 直接作為參數傳入）
--- mud.ask_llm("翻譯：hello world", function(result)
---     mud.echo("翻譯結果: " .. result)
--- end)
--- ```
---@param prompt string 送給 LLM 的提示
---@param callback string|fun(result: string) 回覆後執行的程式碼或函數
---@param model? string 模型名稱（預設 claude-haiku-4-5-20251001）
function mud.ask_llm(prompt, callback, model) end

-- ============================================================
-- 事件系統
-- ============================================================

--- 註冊事件處理器（持續觸發）
---
--- 範例：
--- ```lua
--- mud.on("ifarm:mob_killed", function(data)
---     mud.echo("擊殺了: " .. data.line)
--- end, 0)
--- ```
---@param event_name string 事件名稱
---@param callback string|fun(data: table) 事件觸發時執行的程式碼或函數
---@param priority? integer 優先級（數字越小越先執行，預設 0）
---@return integer handler_id 可用於 mud.off() 移除
function mud.on(event_name, callback, priority) end

--- 註冊一次性事件處理器（觸發一次後自動移除）
---@param event_name string 事件名稱
---@param callback string|fun(data: table) 事件觸發時執行的程式碼或函數
---@param priority? integer 優先級（預設 0）
---@return integer handler_id
function mud.once(event_name, callback, priority) end

--- 移除事件處理器
---@param handler_id integer 由 mud.on() 或 mud.once() 返回的 ID
function mud.off(handler_id) end

--- 發射事件（觸發所有該事件的處理器）
---
--- data 會被 JSON 序列化後傳遞給處理器。
---@param event_name string 事件名稱
---@param data? table|string 附加資料（可選）
function mud.emit(event_name, data) end

-- ============================================================
-- 狀態機
-- ============================================================

---@class SmStateTimeout
---@field seconds number 超時秒數
---@field target string 超時後轉移到的狀態名稱

---@class SmStateDef
---@field enter? string 進入狀態時執行的 Lua 程式碼
---@field exit? string 離開狀態時執行的 Lua 程式碼
---@field timeout? SmStateTimeout 超時設定
---@field timeout_secs? number 超時秒數（替代 timeout table 格式）
---@field timeout_goto? string 超時目標狀態（替代 timeout table 格式）

---@class SmTransition
---@field from string 來源狀態
---@field event string 觸發事件
---@field to string 目標狀態

---@class SmDefinition
---@field initial string 初始狀態名稱
---@field states table<string, SmStateDef> 狀態定義
---@field transitions SmTransition[] 轉移規則

--- 定義並啟動狀態機
---@param name string 狀態機名稱（唯一）
---@param definition SmDefinition 狀態機定義
---@return string name 狀態機名稱
function mud.state_machine(name, definition) end

--- 取得狀態機當前狀態
---@param name string 狀態機名稱
---@return string|nil state 當前狀態名稱，不存在則 nil
function mud.sm_current(name) end

--- 觸發狀態機轉移
---@param name string 狀態機名稱
---@param event string 事件名稱
function mud.sm_transition(name, event) end

--- 重置狀態機到初始狀態
---@param name string 狀態機名稱
function mud.sm_reset(name) end

--- 移除狀態機
---@param name string 狀態機名稱
function mud.sm_remove(name) end

-- ============================================================
-- 地圖 / 房間
-- ============================================================

--- 計算房間的雜湊 ID
---@param name string 房間名稱
---@param desc string 房間描述
---@param exits string[] 出口列表
---@param strict? boolean 是否嚴格匹配（預設 true；false 時忽略描述）
---@return string room_id 房間雜湊 ID
function mud.get_room_id(name, desc, exits, strict) end

--- 取得當前所在房間的 ID
---@param strict? boolean 是否嚴格匹配（預設 true）
---@return string|nil room_id
function mud.get_current_room_id(strict) end

---@class RoomInfo
---@field name string 房間名稱
---@field description string 房間描述
---@field exits string[] 出口列表

--- 取得當前房間完整資訊
---@return RoomInfo|nil room
function mud.get_current_room() end

-- ============================================================
-- 按鍵綁定
-- ============================================================

--- 綁定快捷鍵
---@param key_combo string 按鍵組合（如 "ctrl+f1", "alt+a"）
---@param lua_code string 按下時執行的 Lua 程式碼
function mud.bind_key(key_combo, lua_code) end

--- 解除快捷鍵綁定
---@param key_combo string 按鍵組合
function mud.unbind_key(key_combo) end

-- ============================================================
-- 訊息路由
-- ============================================================

---@class RouteDefinition
---@field name string 路由名稱（唯一）
---@field pattern string 正則表達式匹配模式
---@field window string 目標視窗名稱
---@field gag? boolean 是否同時抑制主視窗顯示（預設 false）

--- 新增訊息路由規則
---@param definition RouteDefinition 路由定義
function mud.add_route(definition) end

--- 移除訊息路由規則
---@param name string 路由名稱
function mud.remove_route(name) end

-- ============================================================
-- 全域變數
-- ============================================================

--- 當前觸發的原始訊息（含 ANSI 色碼）
---@type string
message = ""

--- 當前觸發的純淨訊息（已去 ANSI 和 \r）
---@type string
clean_message = ""

--- 正則捕獲組（觸發器匹配時填充）
---@type string[]
captures = {}

-- ============================================================
-- 全域 Lua Hook 函數（由使用者定義）
-- ============================================================

--- 伺服器訊息鉤子 — 每收到一行伺服器訊息時觸發
---
--- 範例：
--- ```lua
--- function on_server_message(msg, clean, is_echo)
---     if clean:find("^你獲得") then
---         mud.echo("{G[戰利品] " .. clean .. "{x")
---     end
--- end
--- ```
---@param msg string 原始訊息（含 ANSI）
---@param clean string 純淨訊息（已去 ANSI 和 \r）
---@param is_echo boolean 是否為本地回顯
function on_server_message(msg, clean, is_echo) end

--- 指令鉤子 — 使用者送出指令時觸發
---@param cmd string 送出的指令
---@param clean_cmd string 同 cmd
---@param is_echo boolean 固定為 false
function on_command(cmd, clean_cmd, is_echo) end

--- 房間偵測鉤子 — 偵測到新房間時觸發
---@param room_id string 房間 ID（strict hash）
---@param room_name string 房間名稱
---@param is_echo boolean 固定為 false
function on_room_detected(room_id, room_name, is_echo) end
