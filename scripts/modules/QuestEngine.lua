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

    QuestEngine.execute_step()
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

--- Hook for server messages (stub, will be populated in later tasks)
function QuestEngine.on_server_message(line, clean_line, is_echo)
    -- Stub: later tasks will add message matching logic here
end

-- Register hook via MudUtils
MudUtils.register_hook("QuestEngine", QuestEngine.on_server_message)

return QuestEngine
