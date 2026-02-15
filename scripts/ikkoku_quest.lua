-- ============================================================
-- IkkokuQuest - 相聚一刻解謎任務自動腳本 (Refactored)
-- ============================================================
-- 使用: /lua IkkokuQuest.start()
-- 停止: /lua IkkokuQuest.stop()
-- 狀態: /lua IkkokuQuest.status()
-- ============================================================

_G.IkkokuQuest = _G.IkkokuQuest or {}

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("IkkokuQuest cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")
local MudNav = require_module("MudNav")
local MudExplorer = require_module("MudExplorer")
local MudCombat = require_module("MudCombat")

-- Hot-reload MudUtils if show_script_usage is missing (update check)
if not MudUtils.show_script_usage then
    for k, _ in pairs(package.loaded) do
        if k:match("MudUtils$") then package.loaded[k] = nil end
    end
    MudUtils = require_module("MudUtils")
end

-- Hot-reload MudNav if reset is missing
if not MudNav.reset then
    for k, _ in pairs(package.loaded) do
        if k:match("MudNav$") then package.loaded[k] = nil end
    end
    MudNav = require_module("MudNav")
end

-- ===== 常數定義 =====
local CONSTANTS = {
    MAX_FIND_LAPS = 5,
    WATCHDOG_TIMEOUT = 300,
}

-- ===== 設定 =====
_G.IkkokuQuest.config = {
    entry_path = "6w;3n",--;enter ikkoku", -- 分離 enter 指令以避免 MudNav 卡住
    max_find_laps = CONSTANTS.MAX_FIND_LAPS,
    debug = false,
}

-- ===== 任務步驟 =====
local QUEST_STEPS = {
    {name="wait_kyokoo",    target="kyokoo",   target_alias="音無響子", path="n;op n;n;w;op n;n", cmds={"talk kyokoo otonashi", "talk kyokoo yes"}, expect="看能不能說服他進來", next="find_otonashi_1"},
    {name="find_otonashi_1", handler="do_otonashi_logic", path="op s;s;e", expect="不要....叫響子出來見我..!!", next="find_kyokoo_2"},
    {name="find_kyokoo_2",   target="kyokoo",   target_alias="音無響子", path="w;n", cmds={"talk kyokoo otonashi"}, expect="也許五代有辦法，你去問他看看吧...", next="find_godai_1"},
    {name="find_godai_1",    target="godai",    target_alias="五代裕作", path="op n;n;e;3s;2w;n;op n;n;2e;n;u;s;2w;op n;n", cmds={"talk godai otonashi"}, expect="也許我奶奶有辦法吧....你去問看看吧..", next="find_yukari"},
    {name="find_yukari",     target="yukari",   target_alias="五代由加莉", path="op s;s;2e;n;d;s;2w;op s;s;s;2e;3n;w;op s;s", cmds={"talk yukari godai", "talk yukari otonashi"}, expect="五代由加莉 把 錦囊 給了你.", next="find_godai_2"},
    {name="find_godai_2",    target="godai",    target_alias="五代裕作", path="op n;n;e;3s;2w;n;op n;n;2e;n;u;s;2w;op n;n", cmds={"gi bag godai"}, expect="我奶奶說可以試著找四谷先生幫忙...不過四谷是個很怪的人喔..", next="talk_yotsuya"},
    {name="talk_yotsuya",    target="yotsuya",  path="squeeze", handler="do_yotsuya_logic", expect="找朱美比較好解決", next="find_akemi_1"},
    {name="find_akemi_1",    target="akemi",    target_alias="朱美", path="squeeze east;s;e;op n;n", cmds={"talk akemi yotsuya"}, expect="那麼你只要給我一瓶茶茶丸的白酒", next="go_keeper"},
    {name="go_keeper",       target="keeper",   target_alias="茶茶丸的老闆", path="op s;s;e;n;d;s;2w;op s;s;s;push door;n;w", cmds={"talk keeper akemi"}, expect="好...你跟我來一下...", next="chachamaru"},
    {name="chachamaru",      target="keeper",   target_alias="茶茶丸的老闆", path="enter chachamaru", cmds={"talk keeper akemi"}, expect="茶茶丸的老闆 把 白酒 給了你", next="find_akemi_2"},
    {name="find_akemi_2",    target="akemi",    target_alias="朱美", path="push door;e;s;enter ikkoku;n;op n;n;2e;n;u;s;w;op n;n", cmds={"gi wine akemi"}, expect="你把 白酒 給了 朱美.", next="find_otonashi_2"},
    {name="find_otonashi_2", target="otonashi", target_alias="響子的爸爸", path="op s;s;e;n;d;s;w;w", handler="do_otonashi_logic", expect="為了感謝你的幫助，這個戒指就送給你吧!!", next="done"},
}
local STEP_BY_NAME = {}
for i, step in ipairs(QUEST_STEPS) do STEP_BY_NAME[step.name] = i end

-- ===== 狀態 =====
_G.IkkokuQuest.state = {
    running = false,
    run_id = 0,
    step_index = 0,
    phase = "idle",
    watchdog_last = 0,
    finding = false, -- 是否由 MudExplorer 接管
}

local function check_run(rid)
    return rid == _G.IkkokuQuest.state.run_id
end

function _G.IkkokuQuest.echo(msg)
    mud.echo("[IkkokuQuest] " .. msg)
end

-- ===== 公開 API =====

function _G.IkkokuQuest.start()
    if _G.IkkokuQuest.state.running then
        _G.IkkokuQuest.echo("⚠️ 任務已在執行中")
        return
    end

    _G.IkkokuQuest.state.running = true
    _G.IkkokuQuest.state.run_id = MudUtils.get_new_run_id()
    _G.IkkokuQuest.state.step_index = 1
    _G.IkkokuQuest.state.phase = "starting"
    _G.IkkokuQuest.state.finding = false
    
    -- 開始 Log
    MudUtils.start_log("ikkoku")

    -- 註冊並檢查物品
    MudUtils.register_quest("IkkokuQuest", _G.IkkokuQuest.stop)
    mud.send("i")

    _G.IkkokuQuest.echo("🚀 啟動相聚一刻任務！")
    
    _G.IkkokuQuest.check_npc_existence(_G.IkkokuQuest.state.run_id)
end

function _G.IkkokuQuest.check_npc_existence(rid)
    if not check_run(rid) then return end
    _G.IkkokuQuest.echo("🔍 正在確認關鍵角色 (otonashi) 是否存在...")
    
    _G.IkkokuQuest.state.phase = "checking_npc"
    mud.send("q otonashi")
    
    -- 如果 1.5 秒內沒收到「不存在」的訊息，就假設沒問題
    MudUtils.safe_timer(1.5, function(new_rid)
        if not check_run(new_rid) then return end
        if _G.IkkokuQuest.state.phase == "checking_npc" then
            _G.IkkokuQuest.echo("✅ 角色確認完畢，繼續執行...")
            _G.IkkokuQuest.start_flow(new_rid)
        end
    end)
end

function _G.IkkokuQuest.start_flow(rid)
    if not check_run(rid) then return end
    -- Recall & Entry
    MudUtils.send_cmds("wa;recall") -- 正式開始前醒來
    MudNav.config.debug = _G.IkkokuQuest.config.debug
    MudUtils.safe_timer(1.5, _G.IkkokuQuest.enter_area)
end

function _G.IkkokuQuest.enter_area(rid)
    if not check_run(rid) then return end
    _G.IkkokuQuest.echo("🚶 前往一刻館...")
    mud.send("wa") -- 行走前醒來
    MudNav.walk(_G.IkkokuQuest.config.entry_path, _G.IkkokuQuest.on_entered)
end

function _G.IkkokuQuest.on_entered()
    if not _G.IkkokuQuest.state.running then return end
    _G.IkkokuQuest.echo("✅ 到達入口，進入一刻館...")
    mud.send("enter ikkoku")
    
    -- 給予一點時間切換場景，然後確認並開始
    MudUtils.safe_timer(1.0, function(rid)
        if not check_run(rid) then return end
        _G.IkkokuQuest.process_step(rid)
    end)
end

function _G.IkkokuQuest.stop()
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    
    s.running = false
    s.phase = "stopped"
    
    -- 取消所有進行中的非同步任務
    MudUtils.get_new_run_id()
    
    MudExplorer.stop()
    MudNav.reset()
    
    -- 停止 Log
    MudUtils.stop_log()

    _G.IkkokuQuest.echo("🛑 任務停止，移至中古書賣場清理紀錄並 Recall...")
    _G.IkkokuQuest.cleanup_and_recall()
end

function _G.IkkokuQuest.cleanup_and_recall()
    -- 1. Recall 回市中心
    MudUtils.send_cmds("wa;recall")
    
    -- 2. 移動到中古書賣場清理紀錄
    MudUtils.safe_timer(1.2, function(rid)
        MudUtils.send_cmds("3w;3s;w")
        
        MudUtils.safe_timer(1.2, function(rid2)
            MudUtils.send_cmds("wa;quests clear")
            _G.IkkokuQuest.echo("✨ 清理完畢，準備 Recall 回城...")
            
            -- 3. 清理後 Recall
            MudUtils.safe_timer(1.0, function(rid3)
                MudUtils.send_cmds("wa;recall")
            end)
        end)
    end)
end

function _G.IkkokuQuest.status()
    local s = _G.IkkokuQuest.state
    _G.IkkokuQuest.echo("📊 狀態: " .. (s.running and "執行中" or "停止"))
    local step = QUEST_STEPS[s.step_index]
    _G.IkkokuQuest.echo("   步驟: " .. (step and step.name or "N/A"))
    if s.finding then
        MudExplorer.status()
    end
end

function _G.IkkokuQuest.reload()
    package.loaded["scripts.ikkoku_quest"] = nil
    require("scripts.ikkoku_quest")
    _G.IkkokuQuest.echo("♻️ 腳本已重新載入")
end

-- ===== 流程控制 =====

-- 自訂 Yotsuya 邏輯
-- 自訂 Yotsuya 邏輯：他會在地洞與 4, 5 號房之間移動
function _G.IkkokuQuest.do_yotsuya_logic(rid)
    if not check_run(rid) then return end
    
    local function try_talk()
        if not check_run(rid) then return end
        _G.IkkokuQuest.echo("📢 嘗試對話 (talk yotsuya godai)")
        mud.send("talk yotsuya godai")
        -- 核心修正：設定 phase 以便主 Hook 的 expect 檢查能生效
        _G.IkkokuQuest.state.phase = "waiting_response"
    end

    local function loop()
        if not check_run(rid) then return end
        local s = _G.IkkokuQuest.state
        if not s.running or s.step_index ~= STEP_BY_NAME["talk_yotsuya"] then 
            return 
        end

        _G.IkkokuQuest.echo("🕵️ 執行 Yotsuya 搜索 (Gap -> Room4 -> Room5)")
        
        -- 1. 在牆縫中嘗試 (假設目前就在牆縫或剛進來)
        try_talk()
        
        -- 2. 去 Room 4 趕人
        MudUtils.safe_timer(1.5, function()
            if not check_run(rid) then return end
            _G.IkkokuQuest.echo("👉 前往 4 號房...")
            mud.send("squeeze east") -- Gap -> Room 4
            MudUtils.safe_timer(1.2, function()
                try_talk()
                mud.send("squeeze") -- Room 4 -> Gap
                
                -- 3. 去 Room 5 趕人
                MudUtils.safe_timer(1.5, function()
                    if not check_run(rid) then return end
                    _G.IkkokuQuest.echo("👉 前往 5 號房...")
                    mud.send("squeeze west") -- Gap -> Room 5
                    MudUtils.safe_timer(1.2, function()
                        try_talk()
                        mud.send("squeeze") -- Room 5 -> Gap
                        
                        -- 4. 循環
                        MudUtils.safe_timer(2.0, loop)
                    end)
                end)
            end)
        end)
    end
    
    loop()
end

-- 自訂 Otonashi 召喚邏輯 (Safe Summon)
function _G.IkkokuQuest.do_otonashi_logic(rid)
    if not check_run(rid) then return end
    _G.IkkokuQuest.echo("🧙 召喚響子爸爸 (Safe Summon)...")

    -- 使用 MudCombat.safe_summon
    -- 目標: "響子的爸爸" (用於偵測逃跑)
    -- 指令: "cast 'summon' otonashi"
    -- 重試: 5次
    MudCombat.safe_summon("響子的爸爸", "cast 'summon' otonashi", {max_retries=5, verify_delay=2.0}, 
        function() -- success_cb
            if not check_run(rid) then return end
            _G.IkkokuQuest.echo("✅ 召喚成功！嘗試對話...")
            -- 嘗試對話
            mud.send("talk otonashi kyokoo")
            -- 切換狀態等待回應 (由 expect 觸發 monitor)
            _G.IkkokuQuest.state.phase = "waiting_response"
        end,
        function() -- fail_cb
             if not check_run(rid) then return end
             _G.IkkokuQuest.echo("❌ 召喚失敗 (重試次數耗盡)")
             _G.IkkokuQuest.stop()
        end
    )
end

function _G.IkkokuQuest.process_step(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    if not step then
        _G.IkkokuQuest.echo("🎉 任務完成！")
        _G.IkkokuQuest.stop()
        return
    end

    _G.IkkokuQuest.echo("📍 步驟 [" .. s.step_index .. "]: " .. step.name .. " (找 " .. tostring(step.target or "自訂") .. ")")
    
    -- [固定路徑模式]
    -- 如果步驟定義了 path，直接行走該路徑，不再進行探索
    if step.path then
        _G.IkkokuQuest.echo("🚶 執行固定路徑: " .. step.path)
        s.phase = "navigating" -- Reset phase to stop any wait loops
        -- 使用 MudNav 行走
        MudNav.walk(step.path, function()
             -- 到達後檢查目標
             local function check_arrival()
                 if not check_run(rid) then return end
                 
                 _G.IkkokuQuest.echo("✅ 到達目的地")
                 
                 -- 執行抵達後的立即指令 (例如 summon)
                 if step.arrival_cmds then
                     for _, cmd in ipairs(step.arrival_cmds) do
                         mud.send(cmd)
                     end
                 end
                 
                 -- 優先檢查 handler
                 if step.handler then
                     local handler_func = _G.IkkokuQuest[step.handler]
                     if handler_func then
                         _G.IkkokuQuest.echo("🔧 執行自訂邏輯: " .. step.handler)
                         handler_func(rid)
                         return
                     else
                         _G.IkkokuQuest.echo("❌ 找不到 Handler: " .. step.handler)
                     end
                 end

                 if step.target then
                     _G.IkkokuQuest.echo("👀 尋找目標: " .. step.target)
                     s.phase = "waiting_for_mob"
                     _G.IkkokuQuest.start_wait_loop(rid)
                 else
                     -- 無目標，直接執行指令
                     _G.IkkokuQuest.echo("執行指令...")
                     for _, cmd in ipairs(step.cmds) do
                         mud.send(cmd)
                     end
                     
                     if step.expect then
                         s.phase = "waiting_response"
                     else
                         _G.IkkokuQuest.advance_step(rid)
                     end
                 end
             end
             
             -- 增加一點延遲確保描述已顯示
             MudUtils.safe_timer(0.5, check_arrival)
        end)
        return
    end

    -- [探索模式]
    -- ... (MudExplorer logic same as before) ...
    -- 設定 MudExplorer
    MudExplorer.config.target = step.target
    MudExplorer.config.max_laps = _G.IkkokuQuest.config.max_find_laps
    
    s.finding = true
    MudExplorer.explore(function(found, target_line)
        s.finding = false
        if found then
            _G.IkkokuQuest.echo("🎯 找到目標！執行指令...")
            for _, cmd in ipairs(step.cmds) do
                mud.send(cmd)
            end
            
            -- 等待觸發 (expect) 或直接下一步
            if step.expect then
                s.phase = "waiting_response"
                -- Hook 會處理推進
            else
                _G.IkkokuQuest.advance_step(rid)
            end
        else
            -- 這裡處理 "未找到" 的特殊邏輯
            if step.name == "go_keeper" then
                _G.IkkokuQuest.echo("🤔 門口沒人，嘗試進入酒吧...")
                mud.send("enter chachamaru")
                -- 強制切換到 chachamaru 步驟
                local next_idx = STEP_BY_NAME["chachamaru"]
                if next_idx then
                    s.step_index = next_idx
                    MudUtils.safe_timer(1.0, _G.IkkokuQuest.process_step)
                    return
                end
            end
            
            _G.IkkokuQuest.echo("❌ 搜尋失敗，任務中止。")
            _G.IkkokuQuest.stop()
        end
    end)
end

function _G.IkkokuQuest.start_wait_loop(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    
    if not s.running then return end
    if s.phase ~= "waiting_for_mob" then 
        if _G.IkkokuQuest.config.debug then
             _G.IkkokuQuest.echo("Wait loop aborted. Phase mismatch: " .. s.phase)
        end
        return 
    end
    
    mud.send("l")
    
    -- 每隔 5 秒再看一次
    MudUtils.safe_timer(5.0, function(new_rid)
        _G.IkkokuQuest.start_wait_loop(new_rid)
    end)
end

function _G.IkkokuQuest.stop(is_success)
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    
    s.running = false
    s.phase = "stopped"
    
    -- 取消所有進行中的非同步任務
    MudUtils.get_new_run_id()
    
    MudExplorer.stop()
    MudNav.reset()
    
    -- 停止 Log
    MudUtils.stop_log()

    if is_success then
        _G.IkkokuQuest.echo("🛑 任務圓滿完成，準備回城。")
        MudUtils.send_cmds("wa;recall")
    else
        _G.IkkokuQuest.echo("🛑 任務中止，移至中古書賣場清理紀錄並 Recall...")
        _G.IkkokuQuest.cleanup_and_recall()
    end
end

function _G.IkkokuQuest.advance_step(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    
    -- 根據 QUEST_STEPS 的 next 跳轉
    local current_step = QUEST_STEPS[s.step_index]
    local next_name = current_step.next
    
    if next_name == "done" then
        _G.IkkokuQuest.echo("🎉 恭喜！任務全部完成。")
        _G.IkkokuQuest.stop(true) -- 傳入 true 表示成功
        return
    end
    
    local next_idx = STEP_BY_NAME[next_name]
    if next_idx then
        s.step_index = next_idx
        MudUtils.safe_timer(1.0, _G.IkkokuQuest.process_step)
    else
        _G.IkkokuQuest.echo("⚠️ 未知下一步: " .. tostring(next_name))
        _G.IkkokuQuest.stop()
    end
end

-- ===== Server Hook =====
-- 為了避免重複包裝 (Nesting)，我們需要更謹慎地處理 Hook
-- 如果已經安裝過，先恢復原始 Hook 再重新安裝新版本
if _G.IkkokuQuest.hook_installed and _G.IkkokuQuest._original_hook then
    _G.on_server_message = _G.IkkokuQuest._original_hook
end

-- 紀錄原始 Hook (如果是第一次載入)
if not _G.IkkokuQuest._original_hook then
    _G.IkkokuQuest._original_hook = _G.on_server_message
end

local base_hook = _G.IkkokuQuest._original_hook

_G.on_server_message = function(line, clean_line)
    local status, err = pcall(function()
        if base_hook then base_hook(line, clean_line) end
        if _G.IkkokuQuest and _G.IkkokuQuest.on_server_message then
            _G.IkkokuQuest.on_server_message(line, clean_line)
        end
    end)
    if not status then
        mud.echo("CRITICAL HOOK ERROR: " .. tostring(err))
    end
end
_G.IkkokuQuest.hook_installed = true

function _G.IkkokuQuest.on_server_message(line, clean_line)
    if not _G.IkkokuQuest.state.running then return end

    if _G.IkkokuQuest.config.debug then
        mud.echo("[IkkokuQuest] Hook called. Phase: " .. tostring(_G.IkkokuQuest.state.phase))
    end
    
    -- 委派給 MudExplorer
    if _G.IkkokuQuest.state.finding then
        MudExplorer.on_server_message(clean_line)
    end
    
    -- 委派給 MudCombat (處理 safe_summon)
    MudCombat.on_server_message(clean_line)

    -- 委派給 MudNav
    MudNav.on_server_message(clean_line)
    
    local s = _G.IkkokuQuest.state
    local step = QUEST_STEPS[s.step_index]
    
    -- 檢查 NPC 是否存在 (Pre-check)
    if s.phase == "checking_npc" then
        if string.find(clean_line, "這個名稱並不存在於這個系統當中", 1, true) then
            _G.IkkokuQuest.echo("❌ 錯誤: 關鍵角色 (otonashi) 尚未重生或不存在。任務中止。")
            _G.IkkokuQuest.stop()
            return
        end
    end
    
    -- 檢查等待目標 (Fixed Path Mode)
    if s.phase == "waiting_for_mob" and step and step.target then
        local line_lower = string.lower(clean_line)
        local matched = string.find(line_lower, string.lower(step.target), 1, true)
        
        if not matched and step.target_alias then
            matched = string.find(clean_line, step.target_alias, 1, true)
        end
        
        -- Aggressive Debug
        if _G.IkkokuQuest.config.debug then
            mud.echo("[IkkokuQuest_DEBUG] Phase=" .. s.phase .. " Line='" .. clean_line .. "' Target='" .. step.target .. "' Alias='" .. (step.target_alias or "nil") .. "' Matched=" .. tostring(matched))
        end
        
        if matched then
             _G.IkkokuQuest.echo("🎯 發現目標: " .. step.target)
             s.phase = "acting" -- Lock phase
             
             -- Exec cmds
             local rid = s.run_id -- capture current run id
             for _, cmd in ipairs(step.cmds) do 
                 if check_run(rid) then mud.send(cmd) end
             end
             
             if not check_run(rid) then return end
             
             if step.expect then
                 s.phase = "waiting_response"
             else
                 _G.IkkokuQuest.advance_step(rid)
             end
        end
    end
    
    -- 檢查 expect
    if s.phase == "waiting_response" and step and step.expect then
        if string.find(clean_line, step.expect, 1, true) then
            _G.IkkokuQuest.echo("✨ 觸發劇情: " .. step.expect)
            s.phase = "acting"
            _G.IkkokuQuest.advance_step(s.run_id)
        end
    end
end

-- ===== 顯示使用說明 =====
MudUtils.show_script_usage("IkkokuQuest", {
    "IkkokuQuest.start()   - 🚀 啟動相聚一刻任務",
    "IkkokuQuest.stop()    - 🛑 停止任務",
    "IkkokuQuest.status()  - 📊 查看狀態",
    "IkkokuQuest.reload()  - ♻️ 重新載入腳本"
})

return _G.IkkokuQuest
