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
            QuestEngine.advance()
        end
    end

    -- Hunt: combat detection (only during hunt phases)
    if s.hunt_step and (s.phase == "finding" or s.phase == "fighting" or s.phase == "clearing" or s.phase == "looting") then
        local MudCombat = require_module("MudCombat")

        -- Detect kills
        if s.phase == "fighting" and string.find(text, "魂歸西天了") then
            if string.find(text, s.hunt_step.target, 1, true) then
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
                s.non_target_combat = not string.find(text, s.hunt_step.target, 1, true)
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
            if s.hunt_step.target and string.find(text, s.hunt_step.target, 1, true) and string.find(text, "離開了") then
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
            s.phase = "waiting_response"
            s.current_expect = step.expect
            MudUtils.safe_timer(10.0, function()
                if s.running and s.phase == "waiting_response" then
                    mud.echo("[QuestEngine] Timeout waiting for: " .. step.expect)
                    QuestEngine.stop(false)
                end
            end)
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

    -- Configure explorer
    MudExplorer.config.target = step.target
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
                mud.send(step.attack_cmd)
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

    if not s.recovering and s.hunt_step then
        if s.non_target_combat then
            mud.send("flee")
        else
            mud.send(s.hunt_step.attack_cmd)
        end
    end

    MudUtils.safe_timer(2.5, function()
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
        s.phase = "waiting_response"
        s.current_expect = step.expect
        MudUtils.safe_timer(10.0, function()
            if s.running and s.phase == "waiting_response" then
                mud.echo("[QuestEngine] Give timeout: " .. step.expect)
                QuestEngine.stop(false)
            end
        end)
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
        s.phase = "waiting_response"
        s.current_expect = step.expect
        MudUtils.safe_timer(10.0, function()
            if s.running and s.phase == "waiting_response" then
                mud.echo("[QuestEngine] Say timeout: " .. step.expect)
                QuestEngine.stop(false)
            end
        end)
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
        s.phase = "waiting_response"
        s.current_expect = step.expect
        MudUtils.safe_timer(step.timeout or 10.0, function()
            if s.running and s.phase == "waiting_response" then
                if step.retry and (s.interact_retries or 0) < step.retry then
                    s.interact_retries = (s.interact_retries or 0) + 1
                    QuestEngine.execute_step()
                else
                    mud.echo("[QuestEngine] Interact timeout: " .. step.expect)
                    QuestEngine.stop(false)
                end
            end
        end)
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end

return QuestEngine
