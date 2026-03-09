-- scripts/modules/ItemFarmExecutor.lua
-- Job Executor SM for ItemFarm v3
-- Manages single job execution: search -> travel -> engage -> loot -> store -> done

local string = string
local ipairs = ipairs

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("ItemFarmExecutor: cannot load " .. name)
end

local MudNav = require_module("MudNav")
local MudLoot = require_module("MudLoot")
local MudUtils = require_module("MudUtils")
local ItemFarmEngage = require_module("ItemFarmEngage")

local M = {}

local send_cmds = MudUtils.send_cmds

-- Deep merge: b overrides a, field by field
local function deep_merge(a, b)
    local result = {}
    for k, v in pairs(a) do
        if type(v) == "table" and type(b[k]) == "table" then
            result[k] = deep_merge(v, b[k])
        else
            result[k] = v
        end
    end
    for k, v in pairs(b) do
        if result[k] == nil then
            result[k] = v
        elseif type(result[k]) ~= "table" or type(v) ~= "table" then
            result[k] = v
        end
    end
    return result
end

-- Start executing a job
-- on_done(reason): called when job finishes ("done" or "not_found" or "error")
function M.start(job, defaults, on_done)
    -- Merge resources with defaults
    local merged_resources = deep_merge(defaults.resources or {}, job.resources or {})
    local merged_loot = deep_merge(defaults.loot or {}, job.loot or {})
    local merged_store = deep_merge(defaults.store or {}, job.store or {})

    -- Store context
    _G.ItemFarm.executor = {
        job = job,
        defaults = defaults,
        merged_resources = merged_resources,
        merged_loot = merged_loot,
        merged_store = merged_store,
        on_done = on_done,
    }

    local echo = function(msg)
        if _G.ItemFarm and _G.ItemFarm.echo then
            _G.ItemFarm.echo("[" .. job.name .. "] " .. msg)
        end
    end
    _G.ItemFarm.executor.echo = echo

    -- Terminal state helper: echo, consume on_done, call it
    _G.ItemFarm.executor.finish = function(reason, msg)
        if not _G.ItemFarm or not _G.ItemFarm.executor then return end
        _G.ItemFarm.executor.echo(msg)
        local fn = _G.ItemFarm.executor.on_done
        _G.ItemFarm.executor.on_done = nil
        if fn then fn(reason) end
    end

    -- Build SM states
    local search_cmd = job.search.cmd
    local states = {
        searching = {
            enter = function()
                _G.ItemFarm.executor.echo("Searching for target...")
                mud.send("wa")
                mud.send(search_cmd)
            end,
            timeout_secs = 5.0,
            timeout_goto = "not_found",
        },
        traveling = {
            enter = function()
                _G.ItemFarm.executor.echo("Traveling to target...")
                _G.ItemFarm.executor.do_travel()
            end,
            timeout_secs = 120.0,
            timeout_goto = "failed",
        },
        engaging = {
            enter = function()
                _G.ItemFarm.executor.echo("Engaging target...")
                _G.ItemFarm.executor.do_engage()
            end,
        },
        looting = {
            enter = function()
                _G.ItemFarm.executor.echo("Looting...")
                _G.ItemFarm.executor.do_loot()
            end,
            timeout_secs = 10.0,
            timeout_goto = "storing",
        },
        storing = {
            enter = function()
                _G.ItemFarm.executor.echo("Going to storage...")
                _G.ItemFarm.executor.do_store()
            end,
            timeout_secs = 120.0,
            timeout_goto = "done",
        },
        done = {
            enter = function() _G.ItemFarm.executor.finish("done", "Job complete!") end,
        },
        not_found = {
            enter = function() _G.ItemFarm.executor.finish("not_found", "Target not found") end,
        },
        failed = {
            enter = function() _G.ItemFarm.executor.finish("error", "Job failed") end,
        },
        low_resources = {
            enter = function() _G.ItemFarm.executor.finish("low_resources", "Resources insufficient, need rest") end,
        },
    }

    local transitions = {
        { from = "searching",  event = "found",         to = "traveling" },
        { from = "traveling",  event = "arrived",       to = "engaging" },
        { from = "engaging",   event = "engage_done",   to = "looting" },
        { from = "engaging",   event = "engage_failed", to = "failed" },
        { from = "engaging",   event = "engage_low_resources", to = "low_resources" },
        { from = "looting",    event = "loot_done",     to = "storing" },
        { from = "storing",    event = "store_done",    to = "done" },
        -- Failures
        { from = "searching",  event = "fail",          to = "failed" },
        { from = "traveling",  event = "fail",          to = "failed" },
    }

    -- Install action functions
    M.setup_actions(job, defaults, merged_resources, merged_loot, merged_store)

    -- Register event handlers
    M.register_event_handlers(job)

    -- Create SM (reset triggers initial state's enter callback)
    mud.state_machine("itemfarm_job", {
        initial = "searching",
        states = states,
        transitions = transitions,
    })
    mud.sm_reset("itemfarm_job")
end

function M.register_event_handlers(job)
    local td = job.engage.target_display
    -- Search found
    mud.on("ifarm:search_found", function(data)
        local cur = mud.sm_current("itemfarm_job")
        if cur ~= "searching" then return end
        local d = data or {}
        -- For "locate" type, also need to match target
        if job.search.type == "locate" then
            local line = d.line or ""
            local matched = false
            if type(td) == "table" then
                for _, kw in ipairs(td) do
                    if string.find(line, kw, 1, true) then matched = true; break end
                end
            else
                matched = string.find(line, tostring(td), 1, true) ~= nil
            end
            if not matched then return end
        end
        mud.sm_transition("itemfarm_job", "found")
    end, 0)
end

-- Action implementations stored on _G.ItemFarm.executor

function M.setup_actions(job, defaults, merged_resources, merged_loot, merged_store)
    local ex = _G.ItemFarm.executor

    ex.do_travel = function()
        -- Pre-travel command
        if job.travel and job.travel.pre_cmd then
            send_cmds(job.travel.pre_cmd)
        end
        local path = job.travel and job.travel.path or "recall"
        MudNav.walk(path, function(success)
            if success == false then
                mud.sm_transition("itemfarm_job", "fail")
            else
                mud.sm_transition("itemfarm_job", "arrived")
            end
        end)
    end

    ex.do_engage = function()
        ItemFarmEngage.start(job, merged_resources)
    end

    ex.do_loot = function()
        local items = merged_loot.items or {}
        local sac = merged_loot.sac
        if #items == 0 and not sac then
            mud.sm_transition("itemfarm_job", "loot_done")
            return
        end
        MudLoot.process_loot({
            items = items,
            sac = sac,
            loot_ground = true,
            fallback_blind = true,
        }, function()
            mud.sm_transition("itemfarm_job", "loot_done")
        end)
    end

    ex.do_store = function()
        local path = merged_store.path or "recall;3n;e"
        local loot_items = merged_loot.items or {}
        local remove_items = merged_loot.remove_nodrop or {}
        MudNav.walk(path, function(success)
            if success == false then
                mud.sm_transition("itemfarm_job", "store_done")
                return
            end
            -- Remove nodrop + drop items
            for _, item in ipairs(remove_items) do
                mud.send("c 'remove n' " .. item)
            end
            -- Delay drop if we had remove_nodrop
            local delay = #remove_items > 0 and 1.5 or 0
            mud.timer(delay, function()
                if mud.sm_current("itemfarm_job") == "storing" then
                    for _, item in ipairs(loot_items) do
                        mud.send("dro " .. item)
                    end
                    mud.sm_transition("itemfarm_job", "store_done")
                end
            end)
        end)
    end
end

function M.cleanup()
    ItemFarmEngage.cleanup()
    if _G.ItemFarm then
        _G.ItemFarm.executor = nil
    end
end

return M
