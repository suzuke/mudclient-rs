-- scripts/modules/ItemFarmEngage.lua
-- Composable Engage Pipeline for ItemFarm v3
--
-- Dynamically builds a state machine from job config with phases:
--   verify_mob -> dispelling -> check_status -> buffing -> core_action -> kill_confirmed
-- Each phase is included only when the job config requires it.

local string = string
local table = table
local ipairs = ipairs
local tonumber = tonumber
local math = math

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("ItemFarmEngage: cannot load " .. name)
end

local MudCombat = require_module("MudCombat")
local MudNav = require_module("MudNav")
local MudUtils = require_module("MudUtils")

local M = {}

-------------------------------------------------------------------------------
-- Helpers
-------------------------------------------------------------------------------

local send_cmds = MudUtils.send_cmds

-- Check if any target_display keyword matches a line (plain text, no corpse)
local function match_target(line, target_display)
    if not target_display or not line then return false end
    if type(target_display) == "string" then
        return string.find(line, target_display, 1, true) ~= nil
    end
    for _, kw in ipairs(target_display) do
        if string.find(line, kw, 1, true) then return true end
    end
    return false
end

-- Check if a line matches target_display (excluding corpses)
local function match_target_no_corpse(line, td)
    if not line or not td then return false end
    local matched = false
    if type(td) == "table" then
        for _, kw in ipairs(td) do
            if string.find(line, kw, 1, true) then matched = true; break end
        end
    elseif type(td) == "string" then
        matched = string.find(line, td, 1, true) ~= nil
    end
    return matched
        and not string.find(line, "屍體", 1, true)
        and not string.find(line, "corpse", 1, true)
end

-------------------------------------------------------------------------------
-- M.start() — Build and start the engage pipeline SM
-------------------------------------------------------------------------------

function M.start(job, merged_resources)
    -- Initialize engage context on _G.ItemFarm.engage
    _G.ItemFarm = _G.ItemFarm or {}
    _G.ItemFarm.engage = _G.ItemFarm.engage or {}
    local e = _G.ItemFarm.engage
    e.job = job
    e.resources = merged_resources
    e.active_spells = e.active_spells or {}
    e.dispel_retries = 0
    e.charm_retries = 0

    -- Install helper functions
    M.setup_helpers(job, merged_resources)

    -- Build ordered phase list + states + transitions
    local states = {}
    local transitions = {}
    local order = {}

    local mode = job.engage.mode or "direct"
    local charm_max = job.engage.charm_retries or 5
    local td = job.engage.target_display

    ---------------------------------------------------------------------------
    -- Phase: verify_mob (skip for summon modes — safe_summon handles room check)
    ---------------------------------------------------------------------------
    if mode ~= "summon" and mode ~= "summon_charm" then
        order[#order + 1] = "verify_mob"
        states.verify_mob = {
            enter = function()
                _G.ItemFarm.engage.echo("Confirming target in room...")
                mud.collect_response("l", function(lines)
                    for _, line in ipairs(lines) do
                        if match_target_no_corpse(line, td) then
                            if mud.sm_current("itemfarm_engage") == "verify_mob" then
                                mud.sm_transition("itemfarm_engage", "mob_sighted")
                            end
                            return
                        end
                    end
                    -- Not found in room; transition to verify_loc
                    if mud.sm_current("itemfarm_engage") == "verify_mob" then
                        mud.sm_transition("itemfarm_engage", "mob_not_here")
                    end
                end)
            end,
            timeout_secs = 5.0,
            timeout_goto = "verify_loc",
        }

        -- verify_loc: fallback — send search_cmd to check if mob is alive elsewhere
        states.verify_loc = {
            enter = function()
                _G.ItemFarm.engage.echo("Target not in room, checking status...")
                mud.collect_response(job.search.cmd, function(lines)
                    local found = false
                    for _, line in ipairs(lines) do
                        if string.find(line, "他正在這個世界中", 1, true)
                           or string.find(line, "攜帶著", 1, true) then
                            found = true
                            break
                        end
                    end
                    if mud.sm_current("itemfarm_engage") ~= "verify_loc" then return end
                    if found then
                        _G.ItemFarm.engage.echo("Mob alive elsewhere, failing engage")
                    else
                        _G.ItemFarm.engage.echo("Mob not found at all, failing engage")
                    end
                    mud.sm_transition("itemfarm_engage", "fail")
                end)
            end,
            timeout_secs = 5.0,
            timeout_goto = "failed",
        }
    end

    ---------------------------------------------------------------------------
    -- Phase: dispelling
    ---------------------------------------------------------------------------
    if job.engage.dispel then
        local dispel_cmd = job.engage.dispel.cmd
        local max_retries = job.engage.dispel.max_retries or 10

        order[#order + 1] = "dispelling"
        states.dispelling = {
            enter = function()
                _G.ItemFarm.engage.echo("Dispelling...")
                _G.ItemFarm.engage.dispel_retries = 0
                mud.send(dispel_cmd)
                mud.timer(1.5, function()
                    if mud.sm_current("itemfarm_engage") == "dispelling" then
                        _G.ItemFarm.engage.check_dispel()
                    end
                end)
            end,
            timeout_secs = 8.0,
            timeout_goto = "dispel_check",
        }

        -- dispel_check: retry loop
        states.dispel_check = {
            enter = function()
                local eng = _G.ItemFarm.engage
                eng.dispel_retries = eng.dispel_retries + 1
                if eng.dispel_retries >= max_retries then
                    eng.echo("Dispel failed " .. max_retries .. " times, aborting")
                    mud.sm_transition("itemfarm_engage", "fail")
                else
                    eng.echo("Dispel retry " .. eng.dispel_retries .. "/" .. max_retries)
                    mud.send(dispel_cmd)
                    mud.timer(1.5, function()
                        if mud.sm_current("itemfarm_engage") == "dispel_check" then
                            _G.ItemFarm.engage.check_dispel()
                        end
                    end)
                end
            end,
            timeout_secs = 8.0,
            timeout_goto = "dispel_check",  -- loop until max
        }
    end

    ---------------------------------------------------------------------------
    -- Phase: check_status (HP/MP thresholds)
    ---------------------------------------------------------------------------
    if (merged_resources.hp_threshold and merged_resources.hp_threshold > 0)
       or (merged_resources.mp_threshold and merged_resources.mp_threshold > 0) then
        order[#order + 1] = "check_status"
        states.check_status = {
            enter = function()
                _G.ItemFarm.engage.echo("Checking HP/MP status...")
                mud.send("rep")
                mud.send("score aff")
            end,
            timeout_secs = 5.0,
            timeout_goto = "check_status",  -- retry on timeout
        }
    end

    ---------------------------------------------------------------------------
    -- Phase: buffing
    ---------------------------------------------------------------------------
    if merged_resources.buffs and #merged_resources.buffs > 0 then
        order[#order + 1] = "buffing"
        states.buffing = {
            enter = function()
                _G.ItemFarm.engage.apply_next_buff()
            end,
            timeout_secs = 30.0,
            timeout_goto = "buffing",  -- retry
        }
    end

    ---------------------------------------------------------------------------
    -- Core action phases (mode-dependent)
    ---------------------------------------------------------------------------
    if mode == "summon" then
        order[#order + 1] = "summoning"
        states.summoning = {
            enter = function()
                _G.ItemFarm.engage.echo("Summoning target...")
                _G.ItemFarm.engage.start_summon()
            end,
            timeout_secs = 30.0,
            timeout_goto = "failed",
        }
        order[#order + 1] = "fighting"
        states.fighting = {
            enter = function()
                _G.ItemFarm.engage.echo("Fighting!")
                _G.ItemFarm.engage.send_attack()
            end,
            timeout_secs = 60.0,
            timeout_goto = "failed",
        }
    elseif mode == "direct" then
        order[#order + 1] = "fighting"
        states.fighting = {
            enter = function()
                _G.ItemFarm.engage.echo("Fighting!")
                _G.ItemFarm.engage.send_attack()
            end,
            timeout_secs = 60.0,
            timeout_goto = "failed",
        }
    elseif mode == "charm" then
        order[#order + 1] = "charming"
        states.charming = {
            enter = function()
                _G.ItemFarm.engage.echo("Charming target...")
                _G.ItemFarm.engage.charm_retries = 0
                _G.ItemFarm.engage.send_attack()
            end,
            timeout_secs = 3.0,
            timeout_goto = "charm_retry",
        }
        states.charm_retry = {
            enter = function()
                local eng = _G.ItemFarm.engage
                eng.charm_retries = (eng.charm_retries or 0) + 1
                if eng.charm_retries > charm_max then
                    eng.echo("Charm failed " .. charm_max .. " times, aborting")
                    mud.sm_transition("itemfarm_engage", "fail")
                else
                    eng.echo("Charm retry " .. eng.charm_retries .. "/" .. charm_max)
                    eng.send_attack()
                end
            end,
            timeout_secs = 3.0,
            timeout_goto = "charm_retry",
        }

        order[#order + 1] = "leading"
        states.leading = {
            enter = function()
                _G.ItemFarm.engage.echo("Leading charmed target to kill zone...")
                _G.ItemFarm.engage.lead_to_kill()
            end,
            timeout_secs = 60.0,
            timeout_goto = "failed",
        }
        if job.engage.pre_kill_cmds then
            order[#order + 1] = "pre_killing"
            states.pre_killing = {
                enter = function()
                    _G.ItemFarm.engage.echo("Executing pre-kill commands...")
                    _G.ItemFarm.engage.do_pre_kill()
                end,
                timeout_secs = 10.0,
                timeout_goto = "kill_confirmed",
            }
        end
        if job.engage.purge_path then
            order[#order + 1] = "purging"
            states.purging = {
                enter = function()
                    _G.ItemFarm.engage.echo("Walking to purge zone...")
                    _G.ItemFarm.engage.do_purge_walk()
                end,
                timeout_secs = 60.0,
                timeout_goto = "kill_confirmed",
            }
        end
    elseif mode == "summon_charm" then
        -- Summon first, then charm, lead, and wait for kill
        order[#order + 1] = "summoning"
        states.summoning = {
            enter = function()
                _G.ItemFarm.engage.echo("Summoning target...")
                _G.ItemFarm.engage.start_summon()
            end,
            timeout_secs = 30.0,
            timeout_goto = "failed",
        }
        order[#order + 1] = "charming"
        states.charming = {
            enter = function()
                _G.ItemFarm.engage.echo("Charming target...")
                _G.ItemFarm.engage.charm_retries = 0
                _G.ItemFarm.engage.send_attack()
            end,
            timeout_secs = 3.0,
            timeout_goto = "charm_retry",
        }
        states.charm_retry = {
            enter = function()
                local eng = _G.ItemFarm.engage
                eng.charm_retries = (eng.charm_retries or 0) + 1
                if eng.charm_retries > charm_max then
                    eng.echo("Charm failed " .. charm_max .. " times, aborting")
                    mud.sm_transition("itemfarm_engage", "fail")
                else
                    eng.echo("Charm retry " .. eng.charm_retries .. "/" .. charm_max)
                    eng.send_attack()
                end
            end,
            timeout_secs = 3.0,
            timeout_goto = "charm_retry",
        }
        order[#order + 1] = "leading"
        states.leading = {
            enter = function()
                _G.ItemFarm.engage.echo("Leading charmed target to kill zone...")
                _G.ItemFarm.engage.lead_to_kill()
            end,
            timeout_secs = 60.0,
            timeout_goto = "failed",
        }
        if job.engage.pre_kill_cmds then
            order[#order + 1] = "pre_killing"
            states.pre_killing = {
                enter = function()
                    _G.ItemFarm.engage.echo("Executing pre-kill commands...")
                    _G.ItemFarm.engage.do_pre_kill()
                end,
                timeout_secs = 10.0,
                timeout_goto = "kill_confirmed",
            }
        end
        if job.engage.purge_path then
            order[#order + 1] = "purging"
            states.purging = {
                enter = function()
                    _G.ItemFarm.engage.echo("Walking to purge zone...")
                    _G.ItemFarm.engage.do_purge_walk()
                end,
                timeout_secs = 60.0,
                timeout_goto = "kill_confirmed",
            }
        end
    end

    ---------------------------------------------------------------------------
    -- Terminal states (engage → job SM direct transitions)
    ---------------------------------------------------------------------------
    local function terminal_state(job_event, msg)
        return {
            enter = function()
                if not _G.ItemFarm or not _G.ItemFarm.engage then return end
                if msg and msg ~= "" then _G.ItemFarm.engage.echo(msg) end
                local cur = mud.sm_current("itemfarm_job")
                if cur == "engaging" then
                    mud.sm_transition("itemfarm_job", job_event)
                end
            end,
        }
    end
    order[#order + 1] = "kill_confirmed"
    states.kill_confirmed = terminal_state("engage_done", "Kill confirmed!")
    states.low_resources = terminal_state("engage_low_resources", "")
    states.failed = terminal_state("engage_failed", "Engage failed")

    ---------------------------------------------------------------------------
    -- Wire transitions
    ---------------------------------------------------------------------------

    -- Sequential: order[i] --"done"--> order[i+1]
    for i = 1, #order - 1 do
        transitions[#transitions + 1] = { from = order[i], event = "done", to = order[i + 1] }
    end

    -- All non-terminal phases --"fail"--> "failed", --"low_resources"--> "low_resources"
    for _, phase in ipairs(order) do
        if phase ~= "kill_confirmed" and phase ~= "failed" and phase ~= "low_resources" then
            transitions[#transitions + 1] = { from = phase, event = "fail", to = "failed" }
            transitions[#transitions + 1] = { from = phase, event = "low_resources", to = "low_resources" }
        end
    end

    -- verify_mob: mob_sighted -> next phase, mob_not_here -> verify_loc
    if states.verify_mob then
        local next_after_verify = order[2] or "kill_confirmed"
        transitions[#transitions + 1] = { from = "verify_mob", event = "mob_sighted", to = next_after_verify }
        transitions[#transitions + 1] = { from = "verify_mob", event = "mob_not_here", to = "verify_loc" }
    end

    -- dispel: dispel_success -> next phase after dispelling
    if states.dispel_check then
        local next_after_dispel = nil
        for i, p in ipairs(order) do
            if p == "dispelling" and order[i + 1] then
                next_after_dispel = order[i + 1]
                break
            end
        end
        if next_after_dispel then
            transitions[#transitions + 1] = { from = "dispelling", event = "dispel_success", to = next_after_dispel }
            transitions[#transitions + 1] = { from = "dispel_check", event = "dispel_success", to = next_after_dispel }
            transitions[#transitions + 1] = { from = "dispel_check", event = "fail", to = "failed" }
        end
    end

    -- charm: charm_ok -> leading
    if states.charming then
        transitions[#transitions + 1] = { from = "charming", event = "charm_ok", to = "leading" }
        transitions[#transitions + 1] = { from = "charm_retry", event = "charm_ok", to = "leading" }
    end

    -- summoning: summon_ok -> next phase after summoning
    if states.summoning then
        local next_after_summon = nil
        for i, p in ipairs(order) do
            if p == "summoning" and order[i + 1] then
                next_after_summon = order[i + 1]
                break
            end
        end
        transitions[#transitions + 1] = { from = "summoning", event = "summon_ok", to = next_after_summon or "kill_confirmed" }
        -- fighting: target fled/missing -> re-summon (mob walked away)
        if states.fighting then
            transitions[#transitions + 1] = { from = "fighting", event = "resummon", to = "summoning" }
        end
    end

    -- fighting: killed -> kill_confirmed
    if states.fighting then
        transitions[#transitions + 1] = { from = "fighting", event = "killed", to = "kill_confirmed" }
    end

    -- pre_killing/purging transitions are handled by sequential "done" wiring above

    -- Register event handlers that bridge ifarm:* events to SM transitions
    M.register_event_handlers(job)

    -- Create and start the SM
    local initial = order[1]
    mud.state_machine("itemfarm_engage", {
        initial = initial,
        states = states,
        transitions = transitions,
    })
    -- SM creation doesn't run initial state's enter callback, reset to trigger it
    mud.sm_reset("itemfarm_engage")
end

-------------------------------------------------------------------------------
-- M.setup_helpers() — Install helper functions on _G.ItemFarm.engage
-------------------------------------------------------------------------------

function M.setup_helpers(job, merged_resources)
    local e = _G.ItemFarm.engage

    e.echo = function(msg)
        if _G.ItemFarm and _G.ItemFarm.echo then
            _G.ItemFarm.echo("[Engage] " .. msg)
        end
    end

    e.match_target = function(line)
        if not line then return false end
        return match_target(line, job.engage.target_display)
    end

    e.send_attack = function()
        send_cmds(job.engage.attack)
    end

    e.send_cmds_str = function(str)
        send_cmds(str)
    end

    e.start_summon = function()
        MudCombat.safe_summon(job.engage.target_display, job.engage.summon_cmd, {
            max_retries = job.engage.summon_retries or 5, retry_delay = 2.0, verify_delay = 1.0,
        }, function()
            -- success
            mud.sm_transition("itemfarm_engage", "summon_ok")
        end, function()
            -- fail
            mud.sm_transition("itemfarm_engage", "fail")
        end)
    end

    e.lead_to_kill = function()
        -- Send pre-lead commands (or all hi, etc.) before walking
        if job.engage.pre_lead_cmds then
            send_cmds(job.engage.pre_lead_cmds)
        end
        if job.engage.path_to_kill then
            MudNav.walk(job.engage.path_to_kill, function(success)
                if success == false then
                    mud.sm_transition("itemfarm_engage", "fail")
                else
                    mud.sm_transition("itemfarm_engage", "done")
                end
            end)
        else
            mud.sm_transition("itemfarm_engage", "done")
        end
    end

    e.do_pre_kill = function()
        if job.engage.pre_kill_cmds then
            send_cmds(job.engage.pre_kill_cmds)
        end
        -- Brief wait for commands to execute; ifarm:mob_dropped will proceed early
        mud.timer(3.0, function()
            if mud.sm_current("itemfarm_engage") == "pre_killing" then
                mud.sm_transition("itemfarm_engage", "done")
            end
        end)
    end

    e.do_purge_walk = function()
        MudNav.walk(job.engage.purge_path, function()
            if mud.sm_current("itemfarm_engage") == "purging" then
                mud.sm_transition("itemfarm_engage", "done")
            end
        end)
    end

    -- check_dispel: use mud.collect_response("l", ...) to inspect room for indicators
    e.check_dispel = function()
        local indicators = job.engage.dispel and job.engage.dispel.indicators or {}
        local td = job.engage.target_display

        mud.collect_response("l", function(lines)
            local mob_line = nil
            for _, line in ipairs(lines) do
                if match_target_no_corpse(line, td) then
                    mob_line = line
                    break
                end
            end
            local cur = mud.sm_current("itemfarm_engage")
            if cur ~= "dispelling" and cur ~= "dispel_check" then return end
            if not mob_line then return end  -- mob not visible, let timeout handle
            local has_indicator = false
            for _, ind in ipairs(indicators) do
                if string.find(mob_line, ind, 1, true) then
                    has_indicator = true
                    break
                end
            end
            if not has_indicator then
                mud.sm_transition("itemfarm_engage", "dispel_success")
            end
            -- Still has protection: timeout_goto will retry
        end)
    end

    -- evaluate_status: check HP/MP against thresholds
    e.evaluate_status = function()
        local hp_pct = (e.last_hp_max and e.last_hp_max > 0)
            and (e.last_hp / e.last_hp_max * 100) or 100
        local mp_pct = (e.last_mp_max and e.last_mp_max > 0)
            and (e.last_mp / e.last_mp_max * 100) or 100
        local hp_ok = (merged_resources.hp_threshold or 0) == 0
            or hp_pct >= merged_resources.hp_threshold
        local mp_ok = (merged_resources.mp_threshold or 0) == 0
            or mp_pct >= merged_resources.mp_threshold
        if hp_ok and mp_ok then
            mud.sm_transition("itemfarm_engage", "done")
        else
            e.echo(string.format("HP/MP insufficient (HP %.0f%% MP %.0f%%), need rest", hp_pct, mp_pct))
            mud.sm_transition("itemfarm_engage", "low_resources")
        end
    end

    -- apply_next_buff: refresh active_spells via score aff, then apply first missing buff
    e.apply_next_buff = function()
        mud.collect_response("score aff", function()
            if mud.sm_current("itemfarm_engage") ~= "buffing" then return end
            _G.ItemFarm.engage._do_apply_buff()
        end)
    end

    e._do_apply_buff = function()
        local buffs = merged_resources.buffs or {}
        for _, b in ipairs(buffs) do
            local hours = e.active_spells[b.indicator]
            if not hours or hours == 0 then
                e.echo("Applying buff: " .. b.indicator)
                mud.send(b.cmd)
                -- Wait for spell to take effect, then re-check with fresh score aff
                mud.timer(2.0, function()
                    if mud.sm_current("itemfarm_engage") == "buffing" then
                        _G.ItemFarm.engage.apply_next_buff()
                    end
                end)
                return
            end
        end
        -- All buffs present
        e.echo("All buffs active")
        mud.sm_transition("itemfarm_engage", "done")
    end
end

-------------------------------------------------------------------------------
-- M.register_event_handlers() — Bridge ifarm:* events to SM transitions
--
-- All handlers use mud.sm_current() guards since mud.off() is unreliable.
-------------------------------------------------------------------------------

function M.register_event_handlers(job)
    local td = job.engage.target_display
    local mode = job.engage.mode or "direct"

    -- ifarm:mob_killed -> check target match -> transition "killed"
    mud.on("ifarm:mob_killed", function(data)
        local d = data or {}
        local line = d.line or ""
        local cur = mud.sm_current("itemfarm_engage")
        if not cur then return end
        if match_target(line, td) then
            if cur == "fighting" or cur == "waiting_kill" then
                mud.sm_transition("itemfarm_engage", "killed")
            end
        end
    end, 0)

    -- ifarm:mob_fled -> re-summon if summon mode, else fail
    mud.on("ifarm:mob_fled", function()
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "fighting" then
            if mode == "summon" then
                if _G.ItemFarm and _G.ItemFarm.engage then
                    _G.ItemFarm.engage.echo("Target fled, re-summoning...")
                end
                mud.sm_transition("itemfarm_engage", "resummon")
            else
                mud.sm_transition("itemfarm_engage", "fail")
            end
        end
    end, 0)

    -- ifarm:target_missing -> re-summon if summon mode + fighting, else fail
    mud.on("ifarm:target_missing", function()
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "fighting" and mode == "summon" then
            if _G.ItemFarm and _G.ItemFarm.engage then
                _G.ItemFarm.engage.echo("Target missing, re-summoning...")
            end
            mud.sm_transition("itemfarm_engage", "resummon")
        elseif cur == "fighting" or cur == "charming" or cur == "charm_retry" then
            mud.sm_transition("itemfarm_engage", "fail")
        end
    end, 0)

    -- ifarm:status_report -> evaluate HP/MP
    mud.on("ifarm:status_report", function(data)
        local cur = mud.sm_current("itemfarm_engage")
        if cur ~= "check_status" then return end
        local d = data or {}
        _G.ItemFarm.engage.last_hp = d.hp
        _G.ItemFarm.engage.last_hp_max = d.hp_max
        _G.ItemFarm.engage.last_mp = d.mp
        _G.ItemFarm.engage.last_mp_max = d.mp_max
        _G.ItemFarm.engage.evaluate_status()
    end, 0)

    -- ifarm:charm_success -> charm_ok
    mud.on("ifarm:charm_success", function()
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "charming" or cur == "charm_retry" then
            mud.sm_transition("itemfarm_engage", "charm_ok")
        end
    end, 0)

    -- ifarm:spell_list_start -> reset active_spells
    mud.on("ifarm:spell_list_start", function()
        if not _G.ItemFarm or not _G.ItemFarm.engage then return end
        _G.ItemFarm.engage.active_spells = {}
    end, 0)

    -- ifarm:spell_detected -> track active spells
    mud.on("ifarm:spell_detected", function(data)
        if not _G.ItemFarm or not _G.ItemFarm.engage then return end
        local d = data or {}
        if d.name and d.hours then
            _G.ItemFarm.engage.active_spells[d.name] = d.hours
        end
    end, 0)

    -- ifarm:buff_faded -> remove from active_spells, re-check if buffing
    mud.on("ifarm:buff_faded", function(data)
        if not _G.ItemFarm or not _G.ItemFarm.engage then return end
        local d = data or {}
        if d.indicator then
            _G.ItemFarm.engage.active_spells[d.indicator] = nil
        end
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "buffing" then
            _G.ItemFarm.engage.apply_next_buff()
        end
    end, 0)

    -- ifarm:mana_depleted -> fail engage if in buffing/fighting/charming
    mud.on("ifarm:mana_depleted", function()
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "buffing" or cur == "fighting" or cur == "charming" or cur == "charm_retry"
           or cur == "dispelling" or cur == "dispel_check" then
            if _G.ItemFarm and _G.ItemFarm.engage then
                _G.ItemFarm.engage.echo("Mana depleted! Aborting engage")
            end
            mud.sm_transition("itemfarm_engage", "fail")
        end
    end, 0)

    -- ifarm:mob_dropped -> proceed from pre_killing early (item drop confirmed)
    mud.on("ifarm:mob_dropped", function()
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "pre_killing" then
            mud.sm_transition("itemfarm_engage", "done")
        end
    end, 0)
end

-------------------------------------------------------------------------------
-- M.cleanup() — Clear engage context
-------------------------------------------------------------------------------

function M.cleanup()
    if _G.ItemFarm then
        _G.ItemFarm.engage = nil
    end
end

return M
