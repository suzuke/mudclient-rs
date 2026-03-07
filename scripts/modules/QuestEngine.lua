-- QuestEngine Module
-- Declarative quest orchestration engine
-- Sits on top of MudNav, MudExplorer, MudCombat, MudLoot, MudUtils

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("QuestEngine cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

local QuestEngine = _G.QuestEngine or {}
_G.QuestEngine = QuestEngine

-- Sync require cache
for _, key in ipairs({"scripts.modules.QuestEngine", "modules.QuestEngine", "QuestEngine"}) do
    package.loaded[key] = QuestEngine
end

-- Quest definitions stored by name
QuestEngine.quests = QuestEngine.quests or {}

-- Step type handler table (populated by later tasks)
QuestEngine.handlers = QuestEngine.handlers or {}

-- Running state
QuestEngine.state = QuestEngine.state or {
    running = false,
    quest_name = nil,
    step_index = 0,
    run_id = 0,
    phase = nil,
}

--- Define a quest
--- @param name string Quest name
--- @param def table Quest definition with steps array
function QuestEngine.define(name, def)
    QuestEngine.quests[name] = def
end

--- Start a quest by name
--- @param name string Quest name (must be defined first)
function QuestEngine.run(name)
    local def = QuestEngine.quests[name]
    if not def then
        if mud then mud.echo("[QuestEngine] Unknown quest: " .. tostring(name)) end
        return
    end

    -- Stop any currently running quest
    if QuestEngine.state.running then
        QuestEngine.stop(false)
    end

    local rid = MudUtils.get_new_run_id()
    QuestEngine.state = {
        running = true,
        quest_name = name,
        step_index = 1,
        run_id = rid,
        phase = "running",
        expect_id = 0,
    }

    -- Register with MudUtils so halt_all_quests can stop us
    MudUtils.register_quest("QuestEngine", function() QuestEngine.stop(false) end)

    if mud then mud.echo("[QuestEngine] Starting quest: " .. name) end

    if def.recall_cmd then
        mud.send("wa")
        mud.send(def.recall_cmd)
        MudUtils.safe_timer(1.5, function()
            if QuestEngine.state.running then
                QuestEngine.execute_step()
            end
        end)
    else
        QuestEngine.execute_step()
    end
end

--- Stop the current quest
--- @param is_success boolean|nil Whether the quest completed successfully
function QuestEngine.stop(is_success)
    if not QuestEngine.state.running then return end

    local quest_name = QuestEngine.state.quest_name
    -- Invalidate run_id so pending timers become no-ops
    MudUtils.get_new_run_id()

    QuestEngine.state = {
        running = false,
        quest_name = nil,
        step_index = 0,
        run_id = 0,
        phase = nil,
    }

    -- Safely try to stop dependent modules (they may not be loaded)
    pcall(function()
        local MudExplorer = _G.MudExplorer
        if MudExplorer and MudExplorer.stop then MudExplorer.stop() end
    end)
    pcall(function()
        local MudNav = _G.MudNav
        if MudNav and MudNav.stop then MudNav.stop() end
    end)

    -- Unregister from MudUtils
    MudUtils.active_quests["QuestEngine"] = nil

    if mud then
        local status_str = is_success and "completed" or "stopped"
        mud.echo("[QuestEngine] Quest " .. (quest_name or "?") .. " " .. status_str)
    end
end

--- Advance to the next step
function QuestEngine.advance()
    if not QuestEngine.state.running then return end

    local rid = QuestEngine.state.run_id
    QuestEngine.state.step_index = QuestEngine.state.step_index + 1

    -- Brief delay before executing next step
    MudUtils.safe_timer(0.1, function(r)
        if not MudUtils.check_run(r) then return end
        if not QuestEngine.state.running then return end
        QuestEngine.execute_step()
    end)
end

--- Set up an expect with timeout guard using expect_id
--- @param expect string Expected text pattern
--- @param timeout number Timeout in seconds (default 30)
--- @param label string Label for timeout message
function QuestEngine.set_expect(expect, timeout, label)
    local s = QuestEngine.state
    s.expect_id = (s.expect_id or 0) + 1
    s.phase = "waiting_response"
    s.current_expect = expect
    local saved_eid = s.expect_id
    MudUtils.safe_timer(timeout or 30.0, function()
        if s.running and s.phase == "waiting_response" and s.expect_id == saved_eid then
            mud.echo("[QuestEngine] " .. (label or "Expect") .. " timeout (eid=" .. saved_eid .. "): " .. expect)
            QuestEngine.stop(false)
        end
    end)
end

--- Execute the current step
function QuestEngine.execute_step()
    if not QuestEngine.state.running then return end

    local def = QuestEngine.quests[QuestEngine.state.quest_name]
    if not def or not def.steps then
        QuestEngine.stop(false)
        return
    end

    local step = def.steps[QuestEngine.state.step_index]
    if not step then
        -- No more steps: quest complete
        QuestEngine.stop(true)
        return
    end

    local handler = QuestEngine.handlers[step.type]
    if not handler then
        if mud then
            mud.echo("[QuestEngine] Unknown step type: " .. tostring(step.type))
        end
        QuestEngine.stop(false)
        return
    end

    handler(step)
end

--- Hook for server messages
function QuestEngine.on_server_message(line, clean_line, is_echo)
    if is_echo then return end
    if not QuestEngine.state.running then return end
    local s = QuestEngine.state
    local text = clean_line or line

    -- Sleep detection (凍原山頂 auto-sleep)
    if string.find(text, "你正在睡覺") or string.find(text, "睡得很熟") then
        MudUtils.safe_timer(1.5, function() mud.send("wa") end)
        return
    end

    -- Generic expect matching (navigate+cmds, say, interact)
    if s.phase == "waiting_response" and s.current_expect then
        if string.find(text, s.current_expect, 1, true) then
            mud.echo("[QuestEngine] Matched: " .. s.current_expect)
            s.current_expect = nil
            s.expect_id = (s.expect_id or 0) + 1  -- invalidate pending timeout
            QuestEngine.advance()
        end
    end

    -- Hunt: combat detection (only during hunt phases)
    if s.hunt_step and (s.phase == "finding" or s.phase == "fighting" or s.phase == "clearing" or s.phase == "looting") then
        local MudCombat = require_module("MudCombat")
        local lower_text = string.lower(text)
        local lower_target = string.lower(s.hunt_step.target)

        -- Detect kills
        if s.phase == "fighting" and string.find(text, "魂歸西天了") then
            if string.find(lower_text, lower_target, 1, true) then
                s.hunt_kills = s.hunt_kills + 1
                mud.echo("[QuestEngine] Kill #" .. s.hunt_kills)
                s.phase = "clearing"
                s.clear_checks = 0
                MudUtils.safe_timer(1.0, function() QuestEngine._check_combat_clear() end)
                return
            end
        end

        -- Detect item pickup
        if s.hunt_step.loot and s.hunt_step.loot.items then
            for _, item in ipairs(s.hunt_step.loot.items) do
                if string.find(text, item, 1, true) and
                   (string.find(text, "拿出") or string.find(text, "獲得")) then
                    s.hunt_got_item = true
                    mud.echo("[QuestEngine] Got item: " .. item)
                end
            end
        end

        -- Non-target combat detection
        if MudCombat.on_server_message(text) then
            local is_killing_blow = string.find(text, "魂歸西天") or string.find(text, "氣絕")
            if not is_killing_blow and s.phase == "finding" then
                s.non_target_combat = not string.find(lower_text, lower_target, 1, true)
                s.phase = "fighting"
                QuestEngine._combat_heartbeat()
            end
        end

        -- Flee success
        if s.non_target_combat and string.find(text, "你逃離了戰鬥") then
            s.non_target_combat = false
            s.phase = "finding"
            local MudExplorer = require_module("MudExplorer")
            MudExplorer.explore(s.hunt_explore_cb)
            return
        end

        -- Recovery
        if s.phase == "fighting" then
            if string.find(text, "移動力不足") or string.find(text, "法力不足") then
                if not s.recovering then
                    s.recovering = true
                    mud.send("c ref")
                    MudUtils.safe_timer(5.0, function()
                        if s.running and s.recovering then s.recovering = false end
                    end)
                end
            end
            if string.find(text, "體力逐漸地恢復") then
                s.recovering = false
            end

            -- Target fled
            if s.hunt_step.target and string.find(lower_text, lower_target, 1, true) and string.find(text, "離開了") then
                mud.echo("[QuestEngine] Target fled! Resuming exploration...")
                s.recovering = false
                s.phase = "finding"
                local MudExplorer = require_module("MudExplorer")
                MudExplorer.resume(s.hunt_explore_cb)
                return
            end
        end

        -- Body in combat detection
        if string.find(text, "身陷戰鬥中") then
            MudCombat.active()
            if s.phase ~= "fighting" then
                s.phase = "fighting"
                QuestEngine._combat_heartbeat()
            end
        end
    end
end

-- Register hook via MudUtils
MudUtils.register_hook("QuestEngine", QuestEngine.on_server_message)

-- ===== Navigate Handler =====
QuestEngine.handlers["navigate"] = function(step)
    local MudNav = require_module("MudNav")
    local s = QuestEngine.state

    s.phase = "navigating"

    MudNav.walk(step.path, function(success, reason)
        if not s.running or not MudUtils.check_run(s.run_id) then return end

        if not success then
            mud.echo("[QuestEngine] Navigate failed: " .. tostring(reason))
            local on_fail = step.on_fail or "stop"
            if on_fail == "retry" then
                MudUtils.safe_timer(2.0, function() QuestEngine.execute_step() end)
            else
                QuestEngine.stop(false)
            end
            return
        end

        -- Post-arrival commands
        if step.cmds then
            for _, cmd in ipairs(step.cmds) do mud.send(cmd) end
        end

        -- If expect pattern specified, wait for server response
        if step.expect then
            QuestEngine.set_expect(step.expect, step.timeout or 30.0, "Navigate")
            -- Re-trigger room description so expect can match arrival room
            if not step.cmds then mud.send("l") end
        else
            QuestEngine.advance()
        end
    end)
end

-- ===== Hunt Handler =====
-- Flow: explore -> find target -> fight -> loot -> backtrack -> advance
QuestEngine.handlers["hunt"] = function(step)
    local MudExplorer = require_module("MudExplorer")
    local s = QuestEngine.state

    -- Configure explorer (explorer_target avoids matching corpse lines)
    MudExplorer.config.target = step.explorer_target or step.target
    MudExplorer.config.max_laps = step.max_laps or 5
    MudExplorer.config.disable_open_doors = step.disable_open_doors or false
    MudExplorer.config.debug = step.debug or false

    s.phase = "finding"
    s.hunt_step = step
    s.hunt_kills = 0
    s.hunt_got_item = false
    s.non_target_combat = false
    s.recovering = false

    mud.send("wa")

    local function on_explore_done(found, target_line)
        if not s.running or not MudUtils.check_run(s.run_id) then return end

        if found then
            mud.echo("[QuestEngine] Target found! Fighting...")
            s.phase = "fighting"
            s.non_target_combat = false
            mud.send("wa")
            MudUtils.safe_timer(0.5, function()
                if not s.running then return end
                QuestEngine._combat_heartbeat()
            end)
        else
            mud.echo("[QuestEngine] Exploration complete, target not found.")
            QuestEngine.stop(false)
        end
    end

    s.hunt_explore_cb = on_explore_done
    MudExplorer.explore(on_explore_done)
end

function QuestEngine._combat_heartbeat()
    local s = QuestEngine.state
    if not s.running or s.phase ~= "fighting" then return end

    if s.recovering then
        MudUtils.safe_timer(2.5, function()
            QuestEngine._combat_heartbeat()
        end)
        return
    end

    if s.non_target_combat then
        mud.send("flee")
        MudUtils.safe_timer(2.5, function()
            QuestEngine._combat_heartbeat()
        end)
        return
    end

    -- Send attack via collect_response; next round only fires if target still alive
    mud.collect_response(s.hunt_step.attack_cmd, "_G._quest_on_combat_round()")
end

_G._quest_on_combat_round = function()
    local s = QuestEngine.state
    -- If target died, on_server_message already changed phase to "clearing"
    if not s.running or s.phase ~= "fighting" then return end
    -- Target still alive, schedule next attack
    MudUtils.safe_timer(2.0, function()
        QuestEngine._combat_heartbeat()
    end)
end

function QuestEngine._check_combat_clear()
    local s = QuestEngine.state
    if not s.running or s.phase ~= "clearing" then return end

    s.clear_checks = (s.clear_checks or 0) + 1
    local MudCombat = require_module("MudCombat")
    if not MudCombat.is_fighting() or s.clear_checks >= 10 then
        s.clear_checks = 0
        QuestEngine._handle_hunt_loot()
    else
        MudUtils.safe_timer(1.0, function() QuestEngine._check_combat_clear() end)
    end
end

function QuestEngine._handle_hunt_loot()
    local s = QuestEngine.state
    if not s.running then return end
    local MudLoot = require_module("MudLoot")

    local loot = s.hunt_step.loot or {}
    s.phase = "looting"

    MudLoot.process_loot({
        items = loot.items or {"all"},
        loot_ground = loot.loot_ground ~= false,
        sac = loot.sac or false,
        fallback_blind = loot.fallback_blind ~= false,
    }, function()
        QuestEngine._handle_hunt_after_loot()
    end)
end

function QuestEngine._handle_hunt_after_loot()
    local s = QuestEngine.state
    if not s.running then return end
    local MudExplorer = require_module("MudExplorer")
    local MudNav = require_module("MudNav")

    if s.hunt_got_item then
        -- Done hunting, backtrack and advance
        local backtrack = MudExplorer.get_path_to_start()
        MudExplorer.stop()

        if backtrack and backtrack ~= "" then
            mud.echo("[QuestEngine] Backtracking to start...")
            s.phase = "backtracking"
            MudNav.walk(backtrack, function()
                if not s.running then return end
                QuestEngine.advance()
            end)
        else
            QuestEngine.advance()
        end
    else
        -- Continue exploring
        mud.echo("[QuestEngine] No target item yet, resuming exploration...")
        s.phase = "finding"
        MudExplorer.resume(s.hunt_explore_cb)
    end
end

-- ===== Give Handler =====
QuestEngine.handlers["give"] = function(step)
    local s = QuestEngine.state
    mud.send("gi " .. step.item .. " " .. step.npc)

    if step.expect then
        QuestEngine.set_expect(step.expect, step.timeout or 30.0, "Give")
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end

-- ===== Say Handler =====
QuestEngine.handlers["say"] = function(step)
    local s = QuestEngine.state
    local cmd = step.text
    -- Auto-prefix "say" if not already a command
    if not cmd:match("^say ") and not cmd:match("^ta ") and not cmd:match("^talk ") then
        cmd = "say " .. cmd
    end
    mud.send(cmd)

    if step.expect then
        QuestEngine.set_expect(step.expect, step.timeout or 30.0, "Say")
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end

-- ===== Interact Handler =====
QuestEngine.handlers["interact"] = function(step)
    local s = QuestEngine.state
    -- Support single cmd or table of cmds
    if type(step.cmd) == "table" then
        for _, c in ipairs(step.cmd) do mud.send(c) end
    else
        mud.send(step.cmd)
    end

    if step.expect then
        s.expect_id = (s.expect_id or 0) + 1
        s.phase = "waiting_response"
        s.current_expect = step.expect
        local saved_eid = s.expect_id
        MudUtils.safe_timer(step.timeout or 30.0, function()
            if s.running and s.phase == "waiting_response" and s.expect_id == saved_eid then
                if step.retry and (s.interact_retries or 0) < step.retry then
                    s.interact_retries = (s.interact_retries or 0) + 1
                    QuestEngine.execute_step()
                else
                    mud.echo("[QuestEngine] Interact timeout (eid=" .. saved_eid .. "): " .. step.expect)
                    QuestEngine.stop(false)
                end
            end
        end)
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end

-- ===== Summon Handler =====
-- Uses MudCombat.safe_summon, then optionally sends cmds and waits for expect
QuestEngine.handlers["summon"] = function(step)
    local MudCombat = require_module("MudCombat")
    local s = QuestEngine.state

    s.phase = "summoning"

    -- Optional pre-navigate
    if step.path then
        local MudNav = require_module("MudNav")
        MudNav.walk(step.path, function(success)
            if not s.running then return end
            if not success then
                QuestEngine.stop(false)
                return
            end
            QuestEngine._do_summon(step)
        end)
    else
        QuestEngine._do_summon(step)
    end
end

function QuestEngine._do_summon(step)
    local MudCombat = require_module("MudCombat")
    local s = QuestEngine.state

    -- First check if target is already in room via 'l'
    mud.collect_response("l", "_G._quest_summon_check()")
end

-- Callback: check if summon target is already in room
_G._quest_summon_check = function()
    local MudCombat = require_module("MudCombat")
    local s = QuestEngine.state
    if not s.running then return end

    local step = QuestEngine.quests[s.quest_name].steps[s.step_index]
    local lines = _G._collected_lines or {}
    local target_found = false

    for _, line in ipairs(lines) do
        if string.find(line, step.target, 1, true) then
            target_found = true
            break
        end
    end

    if target_found then
        mud.echo("[QuestEngine] Target already here: " .. step.target)
        QuestEngine._summon_success(step)
    else
        QuestEngine._start_summon(step)
    end
end

function QuestEngine._summon_success(step)
    local s = QuestEngine.state
    if not s.running then return end

    if step.cmds then
        for _, cmd in ipairs(step.cmds) do mud.send(cmd) end
    end

    if step.expect then
        QuestEngine.set_expect(step.expect, step.timeout or 30.0, "Summon")
    else
        QuestEngine.advance()
    end
end

function QuestEngine._start_summon(step)
    local MudCombat = require_module("MudCombat")
    local s = QuestEngine.state

    MudCombat.safe_summon(
        step.target,
        step.summon_cmd,
        {
            max_retries = step.max_retries or 5,
            retry_delay = step.retry_delay or 3.0,
            verify_delay = step.verify_delay or 2.0,
        },
        function() -- success
            if not s.running then return end
            mud.echo("[QuestEngine] Summon success: " .. step.target)
            QuestEngine._summon_success(step)
        end,
        function() -- fail
            if not s.running then return end
            mud.echo("[QuestEngine] Summon failed: " .. step.target)
            QuestEngine.stop(false)
        end
    )
end

-- ===== Wait For Mob Handler =====
-- Navigate to location, poll with 'l' until target appears, then send cmds
QuestEngine.handlers["wait_for_mob"] = function(step)
    local s = QuestEngine.state

    local function start_polling()
        if not s.running then return end
        s.phase = "waiting_for_mob"
        s.wait_target = step.target
        s.wait_target_alias = step.target_alias
        s.wait_step = step
        mud.echo("[QuestEngine] Waiting for: " .. (step.target_alias or step.target))
        mud.send("l")
        QuestEngine._wait_poll()

        -- Optional timeout with fallback
        if step.wait_timeout then
            local saved_step = s.step_index
            MudUtils.safe_timer(step.wait_timeout, function()
                if not s.running or s.phase ~= "waiting_for_mob" or s.step_index ~= saved_step then return end
                mud.echo("[QuestEngine] Wait timeout, using fallback")
                s.phase = "acting"
                if step.timeout_cmds then
                    for _, cmd in ipairs(step.timeout_cmds) do mud.send(cmd) end
                end
                if step.timeout_skip then
                    -- Skip N steps forward
                    s.step_index = s.step_index + step.timeout_skip
                    MudUtils.safe_timer(1.0, function()
                        if s.running then QuestEngine.execute_step() end
                    end)
                else
                    QuestEngine.advance()
                end
            end)
        end
    end

    if step.path then
        local MudNav = require_module("MudNav")
        s.phase = "navigating"
        MudNav.walk(step.path, function(success)
            if not s.running then return end
            if not success then
                QuestEngine.stop(false)
                return
            end
            MudUtils.safe_timer(0.5, start_polling)
        end)
    else
        start_polling()
    end
end

function QuestEngine._wait_poll()
    local s = QuestEngine.state
    if not s.running or s.phase ~= "waiting_for_mob" then return end

    MudUtils.safe_timer(5.0, function()
        if not s.running or s.phase ~= "waiting_for_mob" then return end
        mud.send("l")

        -- Optional: attempt summon every other poll cycle
        local step = s.wait_step
        if step and step.summon_cmd then
            s._summon_poll_count = (s._summon_poll_count or 0) + 1
            -- Try summon every 2nd poll (every ~10s)
            if s._summon_poll_count % 2 == 0 then
                MudUtils.safe_timer(1.0, function()
                    if not s.running or s.phase ~= "waiting_for_mob" then return end
                    mud.send(step.summon_cmd)
                end)
            end
        end

        QuestEngine._wait_poll()
    end)
end

-- ===== Custom Handler =====
-- Runs an inline function: step.fn(step, QuestEngine)
-- If step.expect is defined, set it up before calling fn
QuestEngine.handlers["custom"] = function(step)
    if step.expect then
        QuestEngine.set_expect(step.expect, step.timeout or 30.0, "Custom")
    end
    if step.fn then
        step.fn(step, QuestEngine)
    else
        mud.echo("[QuestEngine] Custom step missing fn")
        QuestEngine.stop(false)
    end
end

-- ===== Precheck Support =====
-- Quest-level precheck: list of NPC IDs to verify existence before starting
function QuestEngine._run_precheck(def, callback)
    local npcs = def.precheck
    if not npcs or #npcs == 0 then
        callback()
        return
    end

    local missing = {}
    local index = 0
    local s = QuestEngine.state
    s.phase = "prechecking"
    s.precheck_missing = missing
    s.precheck_npc = nil

    local function check_next()
        if not s.running then return end
        index = index + 1
        if index > #npcs then
            if #missing > 0 then
                mud.echo("[QuestEngine] Missing NPCs: " .. table.concat(missing, ", "))
                mud.echo("[QuestEngine] Retrying in 30s...")
                MudUtils.safe_timer(30.0, function()
                    if not s.running then return end
                    missing = {}
                    s.precheck_missing = missing
                    index = 0
                    check_next()
                end)
            else
                mud.echo("[QuestEngine] Precheck passed")
                callback()
            end
            return
        end

        s.precheck_npc = npcs[index]
        mud.send("q " .. npcs[index])
        MudUtils.safe_timer(0.8, check_next)
    end

    check_next()
end

-- Extend on_server_message for precheck and wait_for_mob
local original_on_server_message = QuestEngine.on_server_message
QuestEngine.on_server_message = function(line, clean_line, is_echo)
    if is_echo then return end
    if not QuestEngine.state.running then return end
    local s = QuestEngine.state
    local text = clean_line or line

    -- Precheck NPC existence detection
    if s.phase == "prechecking" and s.precheck_npc then
        if string.find(text, "這個名稱並不存在於這個系統當中", 1, true) then
            table.insert(s.precheck_missing, s.precheck_npc)
            mud.echo("[QuestEngine] Missing: " .. s.precheck_npc)
        end
    end

    -- Wait for mob detection
    if s.phase == "waiting_for_mob" and s.wait_target then
        local matched = false
        if s.wait_target_alias then
            matched = string.find(text, s.wait_target_alias, 1, true)
        end
        if not matched then
            local bracketed = "(" .. s.wait_target .. ")"
            matched = string.find(string.lower(text), string.lower(bracketed), 1, true)
        end
        -- Also detect summon success: "XXX 突然出現在你的眼前"
        if not matched and s.wait_target_alias then
            local short_name = s.wait_target_alias:match("^([^(]+)") or s.wait_target_alias
            short_name = short_name:gsub("%s+$", "")
            if string.find(text, short_name, 1, true) and string.find(text, "突然出現在你的眼前", 1, true) then
                matched = true
            end
        end
        if matched then
            mud.echo("[QuestEngine] Found: " .. s.wait_target)
            s.phase = "acting"
            local step = s.wait_step
            if step and step.cmds then
                for _, cmd in ipairs(step.cmds) do mud.send(cmd) end
            end
            if step and step.expect then
                QuestEngine.set_expect(step.expect, step.timeout or 30.0, "WaitMob")
            else
                QuestEngine.advance()
            end
            return
        end
    end

    -- Delegate to original handler (sleep detection, expect, hunt)
    original_on_server_message(line, clean_line, is_echo)
end

-- Re-register hook with extended handler
MudUtils.register_hook("QuestEngine", QuestEngine.on_server_message)

-- Override run to support precheck and on_fail
local original_run = QuestEngine.run
QuestEngine.run = function(name)
    local def = QuestEngine.quests[name]
    if not def then
        if mud then mud.echo("[QuestEngine] Unknown quest: " .. tostring(name)) end
        return
    end

    -- Stop any currently running quest
    if QuestEngine.state.running then
        QuestEngine.stop(false)
    end

    local rid = MudUtils.get_new_run_id()
    QuestEngine.state = {
        running = true,
        quest_name = name,
        step_index = 1,
        run_id = rid,
        phase = "running",
        expect_id = 0,
    }

    MudUtils.register_quest("QuestEngine", function() QuestEngine.stop(false) end)

    if mud then mud.echo("[QuestEngine] Starting quest: " .. name) end

    -- Start log if configured
    if def.log_name then
        MudUtils.start_log(def.log_name)
    end

    local function after_precheck()
        if not QuestEngine.state.running then return end
        if def.recall_cmd then
            mud.send("wa")
            mud.send(def.recall_cmd)
            MudUtils.safe_timer(1.5, function()
                if QuestEngine.state.running then
                    QuestEngine.execute_step()
                end
            end)
        else
            QuestEngine.execute_step()
        end
    end

    if def.precheck then
        QuestEngine._run_precheck(def, after_precheck)
    else
        after_precheck()
    end
end

-- Override stop to support on_fail cleanup
local original_stop = QuestEngine.stop
QuestEngine.stop = function(is_success)
    if not QuestEngine.state.running then return end

    local quest_name = QuestEngine.state.quest_name
    local def = QuestEngine.quests[quest_name]

    -- Stop log
    if def and def.log_name then
        MudUtils.stop_log()
    end

    -- Call original stop
    original_stop(is_success)

    -- On success, recall to let NPCs respawn
    if is_success and def and def.recall_cmd then
        MudUtils.safe_timer(1.0, function()
            mud.send(def.recall_cmd)
        end)
    end

    -- On failure, run cleanup if defined
    if not is_success and def and def.on_fail then
        local MudNav = require_module("MudNav")
        MudUtils.safe_timer(1.0, function()
            mud.send("wa")
            mud.send("recall")
            if def.on_fail.path then
                MudUtils.safe_timer(1.5, function()
                    MudNav.walk(def.on_fail.path, function()
                        if def.on_fail.cmds then
                            for _, cmd in ipairs(def.on_fail.cmds) do
                                mud.send(cmd)
                            end
                        end
                    end)
                end)
            end
        end)
    end

    -- Loop mode
    if is_success and def and def.loop then
        mud.echo("[QuestEngine] Loop mode: restarting in 10s...")
        MudUtils.safe_timer(10.0, function()
            QuestEngine.run(quest_name)
        end)
    end
end

return QuestEngine
