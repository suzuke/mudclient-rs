-- ============================================================
-- SmurfQuest - 藍色小精靈解謎任務自動腳本 (Refactored)
-- ============================================================
-- 使用: /lua SmurfQuest.start()
-- 停止: /lua SmurfQuest.stop()
-- 狀態: /lua SmurfQuest.status()
-- ============================================================

_G.SmurfQuest = _G.SmurfQuest or {}

-- Robust require function to handle different CWDs
local function require_module(name)
    local paths = {
        "scripts.modules." .. name,
        "modules." .. name,
        name
    }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("Failed to load module '" .. name .. "'. Please ensure 'scripts/modules/' exists.")
end

local MudUtils = require_module("MudUtils")

local MudNav = require_module("MudNav")
local MudCombat = require_module("MudCombat")
local MudLoot = require_module("MudLoot")

local string = string
local table = table
local os = os

-- ===== 常數定義 =====
local CONSTANTS = {
    TIMER_STEP_COMPLETE = 2.0,
    TIMER_RETRY_SUMMON = 3.0,
    TIMER_WATCHDOG_CHECK = 30.0,
    TIMER_LOOP_RESTART = 10.0,
}

-- ===== 正則表達式 =====
local PATTERNS = {
    SUMMON_FAIL = "你失敗了",
    PAPA_GIVE_KEY = "小精靈老爸 把 小鑰匙 給了你",
    GARGAMEL_DIE = "賈不妙魂歸西天了",
    WAND_TAKEN = "中拿出了 小魔杖",
    PAPA_GIVE_POTION = "小精靈老爸 把 粉紅藥劑 給了你",
    MOB_ALIVE = "他正在這個世界中",
    MOB_NOT_FOUND = "並不存在於這個系統當中",
    TARGET_NOT_HERE = "你想攻擊的對象不在這裡",
}

-- ===== 信號機制 =====
-- Handler 完成時注入信號，統一由 expect 推進
local function signal(name)
    MudUtils.safe_timer(0.3, function(rid)
        if MudUtils.check_run(rid) then
            _G.SmurfQuest.on_server_message("__SIGNAL__:" .. name)
        end
    end)
end

-- ===== 設定 =====
_G.SmurfQuest.config = {
    entry_path = "3w;3s;e;look painting;s;4e;4n", -- 前往村莊入口
    watchdog_timeout = 180,
    debug = true,
}

-- ===== 任務步驟定義 =====
-- 所有步驟統一由 expect 推進，handler 完成後注入 signal 觸發 expect
local QUEST_STEPS = {
    {name="go_entrance",    cmds={"5n;2w;n"}, expect="通往賈不妙的城堡的小徑"},
    {name="summon_papa_1",  cmds={},          expect="__SIGNAL__:summon_papa_1"},
    {name="talk_papa_yes",  cmds={"ta papa yes"}, expect=PATTERNS.PAPA_GIVE_KEY},
    {name="go_castle_gate", cmds={"n"},       expect="賈不妙的城堡外"},
    {name="enter_castle",   cmds={},          expect="賈不妙的城堡"},
    {name="kill_gargamel",  cmds={},          expect="__SIGNAL__:kill_gargamel"},
    {name="get_wand",       cmds={},          expect="__SIGNAL__:get_wand"},
    {name="summon_papa_2",  cmds={},          expect="__SIGNAL__:summon_papa_2"},
    {name="give_wand",      cmds={"gi wand papa"}, expect=PATTERNS.PAPA_GIVE_POTION},
}

-- ===== 狀態變數 =====
_G.SmurfQuest.state = _G.SmurfQuest.state or {
    running = false,
    -- run_id handled by MudUtils
    step_index = 0,
    last_activity = 0,
    watchdog_enabled = false,
    loop_mode = false,
    
    step_completed = false,
    corpse_count = 0,
    looting_active = false,
    check_targets = {},
    check_index = 0,
}

function _G.SmurfQuest.reset_state()
    local s = _G.SmurfQuest.state
    local preserve_loop = s.loop_mode
    s.running = false
    s.step_index = 0
    s.last_activity = os.time()
    s.watchdog_enabled = false
    s.loop_mode = preserve_loop
    s.step_completed = false
    s.corpse_count = 0
    s.looting_active = false
    s.check_targets = {}
    s.check_index = 0
end

-- ===== 訊息處理 =====
function _G.SmurfQuest.echo(msg)
    mud.echo("[SmurfQuest] " .. msg)
end

function _G.SmurfQuest.update_activity()
    _G.SmurfQuest.state.last_activity = os.time()
end

-- ===== 核心輔助 =====
local function match_pattern(text, pattern_key)
    local p = PATTERNS[pattern_key] or pattern_key
    if pattern_key == "EXIT" or pattern_key:find("LEAVE") or pattern_key:find("SQUEEZE") then
        return string.find(text, p)
    else
        return string.find(text, p, 1, true)
    end
end

-- ===== 邏輯函數 =====

function _G.SmurfQuest.recall_and_go(path, callback)
    if not _G.SmurfQuest.state.running then return end
    _G.SmurfQuest.echo("✨ 發送 recall 並準備前往目標...")
    mud.send("recall")
    -- 這裡使用 MudNav 處理 recall 後的延遲和移動
    -- 我們可以在 MudNav 裡加一個 delay_walk? 或者直接用 timer
    MudUtils.safe_timer(1.5, function(rid)
        if not MudUtils.check_run(rid) then return end
        MudNav.walk(path, callback)
        -- Trigger MudNav by looking, in case we missed the room description
        mud.send("l")
    end)
end

function _G.SmurfQuest.perform_check(rid)
    if not MudUtils.check_run(rid) or not _G.SmurfQuest.state.running then return end
    
    local s = _G.SmurfQuest.state
    
    if s.check_index == 0 then
        s.check_targets = {"papa", "gargamel"}
        s.check_index = 1
        _G.SmurfQuest.echo("🔍 啟動任務預檢...")
    end

    if s.check_index > #s.check_targets then
        _G.SmurfQuest.echo("✅ 預檢通過！所有目標已就位。")
        _G.SmurfQuest.recall_and_go(_G.SmurfQuest.config.entry_path, function()
             _G.SmurfQuest.run_step(MudUtils.run_id) 
        end)
        return
    end

    local target = s.check_targets[s.check_index]
    _G.SmurfQuest.echo("🔍 預檢中 [" .. s.check_index .. "/" .. #s.check_targets .. "]: " .. target)
    s.check_waiting = true
    mud.send("q " .. target)

    MudUtils.safe_timer(5.0, function(new_rid)
        if not MudUtils.check_run(new_rid) then return end
        if s.check_waiting then
            _G.SmurfQuest.echo("⏳ 預檢響應超時，30秒後重試...")
            MudUtils.safe_timer(30.0, _G.SmurfQuest.perform_check)
        end
    end)
end

-- Handler Implementations
local step_handlers = {}

function step_handlers.summon_papa_1(rid)
    _G.SmurfQuest.echo("✨ 召喚小精靈老爸 (第一次)...")
    MudCombat.safe_summon("小精靈老爸", "c sum papa", {max_retries=10, retry_delay=3.0, verify_delay=3.0}, 
        function() 
            _G.SmurfQuest.echo("✅ 老爸召喚成功！")
            signal("summon_papa_1")
        end,
        function() 
            _G.SmurfQuest.echo("❌ 召喚失敗次數過多！")
            _G.SmurfQuest.stop()
        end
    )
end

function step_handlers.summon_papa_2(rid)
    _G.SmurfQuest.echo("✨ 向南移動並召喚小精靈老爸 (第二次)...")
    MudNav.walk("s", function()
        MudCombat.safe_summon("小精靈老爸", "c sum papa", {max_retries=10, retry_delay=3.0, verify_delay=3.0}, 
            function() 
                _G.SmurfQuest.echo("✅ 老爸召喚成功！")
                signal("summon_papa_2")
            end,
            function() 
                _G.SmurfQuest.echo("❌ 召喚失敗次數過多！")
                _G.SmurfQuest.stop()
            end
        )
    end)
end



function step_handlers.enter_castle(rid)
    _G.SmurfQuest.echo("🔑 解鎖並進入城堡...")
    mud.send("un n")
    mud.send("op n")
    MudUtils.safe_timer(0.5, function(new_rid)
        if not MudUtils.check_run(new_rid) then return end
        MudNav.walk("n", function()
            _G.SmurfQuest.echo("🏰 進入城堡！")
            -- expect "賈不妙的城堡" 會在 on_server_message 中自動推進
        end)
    end)
end

function step_handlers.kill_gargamel(rid)
    local s = _G.SmurfQuest.state
    _G.SmurfQuest.echo("⚔️ 準備戰鬥：偵測賈不妙是否在場...")
    s.combat_target = "gargamel"
    mud.send("l")
    
    MudUtils.safe_timer(1.0, function(new_rid)
        if not MudUtils.check_run(new_rid) then return end
        if not s.target_found then
            _G.SmurfQuest.echo("❌ 賈不妙不在城堡內！停止任務。")
            _G.SmurfQuest.stop()
        else
            _G.SmurfQuest.echo("💥 賈不妙在場，發動攻擊！")
            mud.send("c sa")
            mud.send("kill gargamel")
            
            -- 技能循環：每 4 秒施放一次，死亡時 expect 會觸發信號推進
            local function combat_loop(loop_rid)
                if not MudUtils.check_run(loop_rid) or not _G.SmurfQuest.state.running then return end
                local current_step = QUEST_STEPS[_G.SmurfQuest.state.step_index]
                if not current_step or current_step.name ~= "kill_gargamel" then return end
                if _G.SmurfQuest.state.step_completed then return end  -- 已擊殺，停止循環
                mud.send("ear gargamel")
                MudUtils.safe_timer(4.0, combat_loop)
            end
            combat_loop(new_rid)
        end
    end)
end

function step_handlers.get_wand(rid)
    _G.SmurfQuest.echo("💰 戰利品階段，等待掉落結算...")
    MudUtils.safe_timer(1.0, function(new_rid)
        if not MudUtils.check_run(new_rid) then return end
        MudLoot.process_loot({
            items = {"wand"},
            fallback_blind = true,
            sac = false
        }, function()
            signal("get_wand")
        end)
    end)
end

function _G.SmurfQuest.run_step(rid)
    if not MudUtils.check_run(rid) or not _G.SmurfQuest.state.running then return end

    local step = QUEST_STEPS[_G.SmurfQuest.state.step_index]
    if not step then return end

    _G.SmurfQuest.state.step_completed = false
    _G.SmurfQuest.state.target_found = false
    _G.SmurfQuest.echo("📋 執行步驟: " .. step.name)
    _G.SmurfQuest.update_activity()

    if step_handlers[step.name] then
        step_handlers[step.name](rid)
        return
    end

    -- Process commands
    local has_move = false
    for _, cmd in ipairs(step.cmds) do
        -- Check if simple move command mostly (checking against generic directions)
        -- MudNav usually strictly for navigation, but here we mix movement and other cmds
        -- If it contains movement, we use MudNav.walk
        -- Note: step.cmds is a list of strings, potentially with semicolons
        if cmd:match("^[nsewud]$") or cmd:match("^[nsewud]%d+") or cmd:match("^%d+[nsewud]$") then
             has_move = true
        end
    end

    if has_move then
        local walk_str = table.concat(step.cmds, ";")
        MudNav.walk(walk_str, function()
            -- After walk, wait for expect or advance
            if not step.expect or step.expect == "" then
                _G.SmurfQuest.advance_step(MudUtils.run_id)
            end
        end)
    else
        -- Just send commands
        local cmds = MudUtils.parse_cmds(table.concat(step.cmds, ";"))
        for _, c in ipairs(cmds) do
            mud.send(c)
        end
        if not step.expect or step.expect == "" then
            MudUtils.safe_timer(CONSTANTS.TIMER_STEP_COMPLETE, _G.SmurfQuest.advance_step)
        end
    end
end

function _G.SmurfQuest.advance_step(rid)
    if not MudUtils.check_run(rid) then return end
    
    local s = _G.SmurfQuest.state
    if s.step_index >= #QUEST_STEPS then
        _G.SmurfQuest.quest_complete(rid)
        return
    end

    s.step_index = s.step_index + 1
    _G.SmurfQuest.run_step(rid)
end

function _G.SmurfQuest.quest_complete(rid)
    _G.SmurfQuest.echo("🎉 藍色小精靈任務完成！")
    MudUtils.stop_log()
    mud.send("recall")
    _G.SmurfQuest.state.running = false
    
    if _G.SmurfQuest.state.loop_mode then
        _G.SmurfQuest.echo("🔄 循環模式：10秒後重新啟動...")
        MudUtils.safe_timer(CONSTANTS.TIMER_LOOP_RESTART, _G.SmurfQuest.init)
    end
end

function _G.SmurfQuest.reload()
    package.loaded["scripts.smurf_quest"] = nil
    require("scripts.smurf_quest")
    _G.SmurfQuest.echo("♻️ 腳本已重新載入")
end

-- ===== Hook Registry =====
MudUtils.register_hook("SmurfQuest", function(line, clean_line)
    _G.SmurfQuest.on_server_message(clean_line)
end)

function _G.SmurfQuest.on_server_message(clean_line)
    local s = _G.SmurfQuest.state
    if not s.running then return end
    
    -- Pre-check Logic
    if s.check_waiting then
        if match_pattern(clean_line, "MOB_ALIVE") then
            if _G.SmurfQuest.config.debug then _G.SmurfQuest.echo("DEBUG: Pattern matched MOB_ALIVE on line: " .. clean_line) end
            s.check_waiting = false
            s.check_index = s.check_index + 1
            MudUtils.safe_timer(0.5, _G.SmurfQuest.perform_check)
            return
        elseif match_pattern(clean_line, "MOB_NOT_FOUND") then
            if _G.SmurfQuest.config.debug then _G.SmurfQuest.echo("DEBUG: Pattern matched MOB_NOT_FOUND on line: " .. clean_line) end
            _G.SmurfQuest.echo("❌ 目標不存在: " .. (s.check_targets[s.check_index] or "unknown"))
            s.check_waiting = false
            -- 如果目標不存在，可能需要重試或停止，這裡先選擇暫停並提示
            _G.SmurfQuest.echo("⚠️ 任務依賴目標缺失，請確認後重新執行。")
            _G.SmurfQuest.stop()
            return
        end
        
        if _G.SmurfQuest.config.debug then
             -- Only log potential matches or interesting lines to avoid spam
             if clean_line:find("他正在") or clean_line:find("不存在") then
                 _G.SmurfQuest.echo("DEBUG: Ignored line during check: " .. clean_line .. " (Hex: " .. MudUtils.string_to_hex(clean_line) .. ")")
             end
        end
    end
    
    -- MudCombat/MudLoot 已透過 Hook Registry 自行接收訊息

    if s.combat_target and (clean_line:find(s.combat_target) or clean_line:lower():find(s.combat_target)) then
         s.target_found = true
         _G.SmurfQuest.update_activity()
    end

    -- 戰鬥時若移動力不足，自動嘗試施展恢復技能
    if clean_line:find("移動力不足") or clean_line:find("精疲力竭") then
        mud.send("c ref")
    end

    -- 非預期戰鬥：自動逃跑並終止任務
    local step = QUEST_STEPS[s.step_index]
    if step and step.name ~= "kill_gargamel" then
        if clean_line:find("你現在正身陷戰鬥中") then
            _G.SmurfQuest.echo("⚠️ 遭遇非預期戰鬥，嘗試逃跑...")
            mud.send("flee")
        end
        if clean_line:find("不顧面子從戰鬥中逃了") then
            _G.SmurfQuest.echo("🏃 逃跑成功，終止任務。")
            _G.SmurfQuest.stop()
            return
        end
    end

    -- 偵測賈不妙死亡 → 發送信號
    if clean_line:find(PATTERNS.GARGAMEL_DIE, 1, true) then
        signal("kill_gargamel")
    end

    -- Step Expectation（統一推進機制）
    local step = QUEST_STEPS[s.step_index]
    if step and step.expect and step.expect ~= "" and not s.step_completed then
        if match_pattern(clean_line, step.expect) then
            _G.SmurfQuest.echo("✨ 達成條件: " .. step.name)
            s.step_completed = true
            _G.SmurfQuest.update_activity()
            MudUtils.safe_timer(0.5, _G.SmurfQuest.advance_step)
        end
    end
end

-- ===== Watchdog =====
function _G.SmurfQuest.watchdog(rid)
    if not MudUtils.check_run(rid) or not _G.SmurfQuest.state.watchdog_enabled then return end
    local s = _G.SmurfQuest.state
    
    if MudCombat.is_fighting() then
        _G.SmurfQuest.update_activity()
    end
    
    local idle = os.time() - s.last_activity
    if idle > _G.SmurfQuest.config.watchdog_timeout then
        _G.SmurfQuest.echo("⚠️ Watchdog 超時！重置任務...")
        _G.SmurfQuest.stop()
        if s.loop_mode then
            MudUtils.safe_timer(5.0, _G.SmurfQuest.init)
        end
        return
    end
    MudUtils.safe_timer(CONSTANTS.TIMER_WATCHDOG_CHECK, _G.SmurfQuest.watchdog)
end

-- ===== Public API =====
function _G.SmurfQuest.init()
    _G.SmurfQuest.reset_state()
    MudUtils.get_new_run_id()
    MudNav.reset()
    
    local s = _G.SmurfQuest.state
    s.running = true
    s.step_index = 1
    s.last_activity = os.time()
    s.watchdog_enabled = true
    
    s.watchdog_enabled = true
    
    _G.SmurfQuest.echo("🚀 啟動藍色小精靈任務 v0.3 (Refactored)")
    MudUtils.start_log("smurf")
    
    -- 註冊並檢查物品
    MudUtils.register_quest("SmurfQuest", _G.SmurfQuest.stop)
    mud.send("i")
    
    -- Delay check to avoid output interleaving with inventory
    MudUtils.safe_timer(1.0, function(rid)
        if MudUtils.check_run(rid) then
            _G.SmurfQuest.perform_check(rid)
        end
    end)
    MudUtils.safe_timer(CONSTANTS.TIMER_WATCHDOG_CHECK, _G.SmurfQuest.watchdog)
end

function _G.SmurfQuest.start()
    _G.SmurfQuest.state.loop_mode = false
    _G.SmurfQuest.init()
end

function _G.SmurfQuest.stop()
    _G.SmurfQuest.state.running = false
    _G.SmurfQuest.state.watchdog_enabled = false
    MudNav.state.walking = false -- Stop nav as well
    _G.SmurfQuest.echo("🛑 任務已停止")
    MudUtils.stop_log()
end

-- ===== 自動執行 =====
MudUtils.print_script_help(
    "SmurfQuest 藍色小精靈任務", 
    "v0.3 (Refactored)", 
    "自動完成藍色小精靈任務 (召喚老爸、殺賈不妙、拿魔杖)",
    {
        {cmd="SmurfQuest.start()", desc="🚀 開始任務"},
        {cmd="SmurfQuest.stop()",  desc="🛑 停止任務"},
        {cmd="SmurfQuest.status()", desc="📊 查看狀態"},
        {cmd="SmurfQuest.reload()", desc="♻️ 重新載入腳本"},
    }
)