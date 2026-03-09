-- scripts/itemfarm_v3.lua
-- ItemFarm v3.0 - Event-Driven Auto Farm System
-- Architecture: Scheduler SM -> Executor SM -> Engage Pipeline SM

_G.ItemFarm = _G.ItemFarm or {}

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("ItemFarm v3: cannot load " .. name)
end

local MudUtils = require_module("MudUtils")
local MudNav = require_module("MudNav")
local ItemFarmJobs = require_module("ItemFarmJobs")
local ItemFarmParser = require_module("ItemFarmParser")
local ItemFarmExecutor = require_module("ItemFarmExecutor")

local string = string
local ipairs = ipairs
local math = math

-- ===== State =====
_G.ItemFarm.state = {
    running = false,
    current_job = 1,
    jobs_checked = 0,
    loot_count = 0,
    show_echo = ItemFarmJobs.defaults.show_echo,
}

-- ===== Jobs reference =====
_G.ItemFarm.jobs = ItemFarmJobs.jobs
_G.ItemFarm.defaults = ItemFarmJobs.defaults

-- ===== Echo helpers =====
function _G.ItemFarm.echo(msg)
    if _G.ItemFarm.state.show_echo then
        mud.echo("[ItemFarm] " .. msg)
    end
end

function _G.ItemFarm.echo_force(msg)
    mud.echo("[ItemFarm] " .. msg)
end

-- ===== Job helpers =====
function _G.ItemFarm.job()
    return _G.ItemFarm.jobs[_G.ItemFarm.state.current_job]
end

local function count_active_jobs()
    local n = 0
    for _, j in ipairs(_G.ItemFarm.jobs) do
        if not j.disabled then n = n + 1 end
    end
    return n
end

local function find_next_active_job(from)
    local total = #_G.ItemFarm.jobs
    for i = 1, total do
        local idx = ((from - 1 + i) % total) + 1
        if not _G.ItemFarm.jobs[idx].disabled then
            return idx
        end
    end
    return nil
end

-- ===== Scheduler SM =====
local function create_scheduler_sm()
    local defaults = _G.ItemFarm.defaults

    mud.state_machine("itemfarm_scheduler", {
        initial = "idle",
        states = {
            idle = {
                enter = function() _G.ItemFarm._scheduler_idle() end,
            },
            executing = {
                enter = function() _G.ItemFarm._scheduler_executing() end,
            },
            rotating = {
                enter = function() _G.ItemFarm._scheduler_rotating() end,
            },
            recovering = {
                enter = function() _G.ItemFarm._scheduler_recovering() end,
            },
            resting = {
                enter = function() _G.ItemFarm._scheduler_resting() end,
            },
            waiting_respawn = {
                enter = function() _G.ItemFarm._scheduler_waiting_respawn() end,
            },
            stopped = {
                enter = function() _G.ItemFarm._scheduler_stopped() end,
            },
        },
        transitions = {
            { from = "idle",              event = "job_found",      to = "executing" },
            { from = "idle",              event = "no_active_jobs", to = "stopped" },
            { from = "executing",         event = "job_done",       to = "recovering" },
            { from = "executing",         event = "job_failed",     to = "recovering" },
            { from = "executing",         event = "not_found",      to = "rotating" },
            { from = "executing",         event = "low_resources",  to = "resting" },
            { from = "recovering",        event = "recovered",      to = "rotating" },
            { from = "resting",           event = "rested",         to = "idle" },
            { from = "rotating",          event = "next_ready",     to = "idle" },
            { from = "rotating",          event = "all_checked",    to = "waiting_respawn" },
            { from = "waiting_respawn",   event = "wake",           to = "idle" },
            -- Emergency stop from any state
            { from = "idle",              event = "stop",           to = "stopped" },
            { from = "executing",         event = "stop",           to = "stopped" },
            { from = "recovering",        event = "stop",           to = "stopped" },
            { from = "rotating",          event = "stop",           to = "stopped" },
            { from = "resting",           event = "stop",           to = "stopped" },
            { from = "waiting_respawn",   event = "stop",           to = "stopped" },
        },
    })
end

-- Scheduler state callbacks
function _G.ItemFarm._scheduler_idle()
    if mud.sm_current("itemfarm_scheduler") ~= "idle" then return end
    local s = _G.ItemFarm.state
    s.jobs_checked = 0

    -- Find first active job starting from current
    local idx = find_next_active_job(s.current_job - 1)
    if not idx then
        _G.ItemFarm.echo_force("No active jobs")
        mud.sm_transition("itemfarm_scheduler", "no_active_jobs")
        return
    end

    s.current_job = idx
    local j = _G.ItemFarm.job()
    _G.ItemFarm.echo("Starting job [" .. idx .. "] " .. j.name)
    mud.sm_transition("itemfarm_scheduler", "job_found")
end

function _G.ItemFarm._scheduler_executing()
    if mud.sm_current("itemfarm_scheduler") ~= "executing" then return end
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    local defaults = _G.ItemFarm.defaults

    -- ItemFarmExecutor.start() internally calls setup_actions() and
    -- register_event_handlers(), so no need to call them separately.
    ItemFarmExecutor.start(j, defaults, function(reason)
        if reason == "done" then
            s.loot_count = s.loot_count + 1
            _G.ItemFarm.echo("Loot count: " .. s.loot_count)
        end
        s.last_result = reason  -- "done", "not_found", "error", "low_resources"
        ItemFarmExecutor.cleanup()
        local event
        if reason == "low_resources" then event = "low_resources"
        elseif reason == "not_found" then event = "not_found"
        else event = (reason == "done") and "job_done" or "job_failed"
        end
        mud.sm_transition("itemfarm_scheduler", event)
    end)
end

function _G.ItemFarm._scheduler_rotating()
    if mud.sm_current("itemfarm_scheduler") ~= "rotating" then return end
    local s = _G.ItemFarm.state
    local active = count_active_jobs()

    -- Mark current job as checked (only if not disabled)
    local j = _G.ItemFarm.job()
    if not j.disabled then
        s.jobs_checked = s.jobs_checked + 1
    end

    if s.jobs_checked >= active then
        _G.ItemFarm.echo("All jobs checked this round")
        mud.sm_transition("itemfarm_scheduler", "all_checked")
        return
    end

    -- Move to next active job
    local idx = find_next_active_job(s.current_job)
    if not idx then
        mud.sm_transition("itemfarm_scheduler", "all_checked")
        return
    end

    s.current_job = idx
    _G.ItemFarm.echo("Rotating to [" .. idx .. "] " .. _G.ItemFarm.jobs[idx].name)
    mud.sm_transition("itemfarm_scheduler", "next_ready")
end

function _G.ItemFarm._scheduler_recovering()
    if mud.sm_current("itemfarm_scheduler") ~= "recovering" then return end
    local defaults = _G.ItemFarm.defaults
    local rest_cmd = defaults.rest_cmd or "sleep"
    local rest_secs = 10  -- short rest between jobs

    local function do_rest()
        if mud.sm_current("itemfarm_scheduler") ~= "recovering" then return end
        _G.ItemFarm.echo("Brief rest " .. rest_secs .. "s...")
        mud.send(rest_cmd)
        mud.timer(rest_secs, function()
            if mud.sm_current("itemfarm_scheduler") == "recovering" then
                mud.send("wa")
                mud.sm_transition("itemfarm_scheduler", "recovered")
            end
        end)
    end

    local s = _G.ItemFarm.state
    if s.last_result == "done" then
        -- Job succeeded: executor's storing state already brought us to storage
        do_rest()
    else
        -- Job failed mid-execution: still at target location, need to walk back
        local rest_path = defaults.store and defaults.store.path or "recall"
        _G.ItemFarm.echo("Returning to storage...")
        MudNav.walk(rest_path, do_rest)
    end
end

function _G.ItemFarm._scheduler_waiting_respawn()
    if mud.sm_current("itemfarm_scheduler") ~= "waiting_respawn" then return end
    local defaults = _G.ItemFarm.defaults
    local rest_path = defaults.store and defaults.store.path or "recall"
    local rest_cmd = defaults.rest_cmd or "sleep"
    local rest_secs = defaults.poll_interval or 30

    _G.ItemFarm.echo("All jobs checked, resting " .. rest_secs .. "s at storage...")
    MudNav.walk(rest_path, function()
        if mud.sm_current("itemfarm_scheduler") ~= "waiting_respawn" then return end
        mud.send(rest_cmd)
        mud.timer(rest_secs, function()
            if mud.sm_current("itemfarm_scheduler") == "waiting_respawn" then
                mud.send("wa")
                mud.sm_transition("itemfarm_scheduler", "wake")
            end
        end)
    end)
end

function _G.ItemFarm._scheduler_resting()
    if mud.sm_current("itemfarm_scheduler") ~= "resting" then return end
    local defaults = _G.ItemFarm.defaults
    local rest_path = defaults.store and defaults.store.path or "recall"
    local rest_cmd = defaults.rest_cmd or "sleep"
    local rest_secs = defaults.poll_interval or 30

    _G.ItemFarm.echo("Low resources, going to rest at storage...")
    MudNav.walk(rest_path, function()
        if mud.sm_current("itemfarm_scheduler") ~= "resting" then return end
        _G.ItemFarm.echo("Resting " .. rest_secs .. "s...")
        mud.send(rest_cmd)
        mud.timer(rest_secs, function()
            if mud.sm_current("itemfarm_scheduler") == "resting" then
                mud.send("wa")
                mud.sm_transition("itemfarm_scheduler", "rested")
            end
        end)
    end)
end

function _G.ItemFarm._scheduler_stopped()
    if mud.sm_current("itemfarm_scheduler") ~= "stopped" then return end
    -- This is the SM-driven stop path (e.g. no_active_jobs -> stop event)
    -- For manual stop, ItemFarm.stop() handles cleanup directly
    _G.ItemFarm.state.running = false
    MudNav.reset()
    ItemFarmExecutor.cleanup()
    mud.sm_remove("itemfarm_engage")
    mud.sm_remove("itemfarm_job")
    _G.ItemFarm.echo_force("Stopped. Total loot: " .. _G.ItemFarm.state.loot_count)
    MudUtils.stop_log()
end

-- ===== Register Parser Hook =====
MudUtils.register_hook("ItemFarm", function(line, clean_line, is_echo)
    if not _G.ItemFarm.state.running then return end
    ItemFarmParser.parse(line, clean_line, is_echo)
end)

-- ===== Register Global Emergency Handler =====
local function register_global_handlers()
    mud.on("ifarm:unexpected_combat", function()
        local cur = mud.sm_current("itemfarm_engage")
        -- Only trigger emergency if not already fighting
        if cur and cur ~= "fighting" and cur ~= "waiting_kill" then
            _G.ItemFarm.echo_force("EMERGENCY: unexpected combat! Fleeing...")
            mud.send("fl")
            mud.send("recall")
            local j = _G.ItemFarm.job()
            if j then j.disabled = true end
            mud.sm_transition("itemfarm_engage", "fail")
        end
    end, -100)  -- High priority (low number = runs first)
end

-- ===== External API =====
function _G.ItemFarm.start()
    if _G.ItemFarm.state.running then
        _G.ItemFarm.echo_force("Already running")
        return
    end

    local s = _G.ItemFarm.state
    s.running = true
    s.loot_count = 0
    s.current_job = 1
    s.jobs_checked = 0

    _G.ItemFarm.echo_force("Starting ItemFarm v3.0 (" .. #_G.ItemFarm.jobs .. " jobs)")
    MudUtils.start_log("itemfarm")
    MudUtils.register_quest("ItemFarm", _G.ItemFarm.stop)

    -- Register buff fade rules from all jobs
    ItemFarmParser.clear_dynamic_rules()
    for _, j in ipairs(_G.ItemFarm.jobs) do
        if j.resources and j.resources.buffs then
            for _, b in ipairs(j.resources.buffs) do
                if b.fade_msg then
                    ItemFarmParser.add_fade_rule(b.fade_msg, b.indicator)
                end
            end
        end
    end

    register_global_handlers()

    -- Inventory check then start scheduler
    MudUtils.start_inventory_check(function()
        _G.ItemFarm.echo("Inventory check passed, starting scheduler...")
        create_scheduler_sm()
        -- SM creation doesn't run initial state's enter callback,
        -- so reset to trigger it
        mud.sm_reset("itemfarm_scheduler")
    end)
end

function _G.ItemFarm.stop()
    if not _G.ItemFarm.state.running then return end
    _G.ItemFarm.state.running = false

    -- 1. Stop MudNav walk immediately
    MudNav.reset()

    -- 2. Clean up executor and engage contexts
    ItemFarmExecutor.cleanup()

    -- 3. Remove all SMs (no more callbacks will fire)
    mud.sm_remove("itemfarm_engage")
    mud.sm_remove("itemfarm_job")
    mud.sm_remove("itemfarm_scheduler")

    -- 4. Final cleanup
    _G.ItemFarm.echo_force("Stopped. Total loot: " .. _G.ItemFarm.state.loot_count)
    MudUtils.stop_log()
end

function _G.ItemFarm.status()
    local s = _G.ItemFarm.state
    _G.ItemFarm.echo_force("=== ItemFarm v3.0 Status ===")
    _G.ItemFarm.echo_force("Running: " .. (s.running and "yes" or "no"))
    _G.ItemFarm.echo_force("Loot count: " .. s.loot_count)

    if s.running then
        _G.ItemFarm.echo_force("Scheduler: " .. (mud.sm_current("itemfarm_scheduler") or "?"))
        _G.ItemFarm.echo_force("Executor:  " .. (mud.sm_current("itemfarm_job") or "-"))
        _G.ItemFarm.echo_force("Engage:    " .. (mud.sm_current("itemfarm_engage") or "-"))
        local j = _G.ItemFarm.job()
        if j then
            _G.ItemFarm.echo_force("Current job: [" .. s.current_job .. "] " .. j.name)
        end
    end

    _G.ItemFarm.echo_force("Jobs:")
    for i, j in ipairs(_G.ItemFarm.jobs) do
        local marker = (i == s.current_job and s.running) and " <" or ""
        local disabled = j.disabled and " [OFF]" or ""
        _G.ItemFarm.echo_force("  [" .. i .. "] " .. j.name .. " (" .. (j.engage and j.engage.mode or "?") .. ")" .. disabled .. marker)
    end
end

function _G.ItemFarm.toggle_job(index)
    local j = _G.ItemFarm.jobs[tonumber(index)]
    if not j then
        _G.ItemFarm.echo_force("Job not found: " .. tostring(index))
        return
    end
    j.disabled = not j.disabled
    _G.ItemFarm.echo_force("[" .. index .. "] " .. j.name .. ": " .. (j.disabled and "DISABLED" or "ENABLED"))
end

function _G.ItemFarm.toggle_echo()
    _G.ItemFarm.state.show_echo = not _G.ItemFarm.state.show_echo
    _G.ItemFarm.echo_force("Echo: " .. (_G.ItemFarm.state.show_echo and "ON" or "OFF"))
end

function _G.ItemFarm.reload()
    package.loaded["scripts.itemfarm_v3"] = nil
    package.loaded["scripts.modules.ItemFarmJobs"] = nil
    package.loaded["scripts.modules.ItemFarmParser"] = nil
    package.loaded["scripts.modules.ItemFarmExecutor"] = nil
    package.loaded["scripts.modules.ItemFarmEngage"] = nil
    require("scripts.itemfarm_v3")
end

-- ===== Init =====
_G.ItemFarm.echo_force("=== ItemFarm v3.0 (Event-Driven) ===")
_G.ItemFarm.echo_force("Commands: ItemFarm.start() / .stop() / .status() / .toggle_job(n) / .toggle_echo()")
_G.ItemFarm.echo_force("Jobs: " .. #_G.ItemFarm.jobs)
for i, j in ipairs(_G.ItemFarm.jobs) do
    local disabled = j.disabled and " [OFF]" or ""
    _G.ItemFarm.echo_force("  [" .. i .. "] " .. j.name .. " (" .. (j.engage and j.engage.mode or "?") .. ")" .. disabled)
end

return _G.ItemFarm
