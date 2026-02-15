-- MudCombat Module
-- Handles combat related logic: Safe Summon, Dispel, etc.

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
    error("MudCombat cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

local MudCombat = {}
_G.MudCombat = MudCombat

MudCombat.state = {
    summon_retries = 0,
    summon_cb_id = 0,
    summon_callbacks = {},
    summon_active = false,
    -- Configuration for current summon
    target = nil,
    cmd = nil,
    opts = nil,
}

-- Safe Summon
-- opts: { max_retries=3, retry_delay=2.0, verify_delay=1.0 }
function MudCombat.safe_summon(target_name, summon_cmd, opts, success_cb, fail_cb)
    local s = MudCombat.state
    s.summon_active = true
    s.target = target_name
    s.cmd = summon_cmd
    -- Default options
    s.opts = opts or {}
    s.opts.max_retries = s.opts.max_retries or 3
    s.opts.retry_delay = s.opts.retry_delay or 2.0
    s.opts.verify_delay = s.opts.verify_delay or 1.0
    
    s.summon_retries = 0
    s.summon_cb_id = s.summon_cb_id + 1
    s.summon_callbacks[s.summon_cb_id] = {success=success_cb, fail=fail_cb}
    
    -- Start summoning
    MudCombat.do_summon(MudUtils.run_id)
end

function MudCombat.do_summon(rid)
    if not MudUtils.check_run(rid) then return end
    local s = MudCombat.state
    if not s.summon_active then return end
    
    if mud then mud.send(s.cmd) end
end

-- Hook handler for summon messages
function MudCombat.on_server_message(clean_line)
    local s = MudCombat.state
    if not s.summon_active then return end
    
    -- Success
    if string.find(clean_line, "突然出現在你的眼前") then
        -- Wait verify_delay to check if fled
        MudUtils.safe_timer(s.opts.verify_delay, function(rid)
            MudCombat.check_summon_success(rid)
        end)
        return
    end
    
    -- Fail (generic fail message? User provided "你失敗了")
    if string.find(clean_line, "你失敗了") then
        s.summon_retries = s.summon_retries + 1
        if s.summon_retries >= s.opts.max_retries then
            MudCombat.finish_summon(false)
        else
            MudUtils.safe_timer(s.opts.retry_delay, function(rid) 
                MudCombat.do_summon(rid) 
            end)
        end
        return
    end
    
    -- Immediate Flee detection (during verify delay usually, but might happen fast)
    if s.target and string.find(clean_line, s.target) and (string.find(clean_line, "離開了") or string.find(clean_line, "逃了")) then
        -- If we catch a flee message, we might need to cancel the pending success check
        -- But easiest is to handle it in check_summon_success by state or re-trigger there.
        -- If we catch it here immediately, we can just trigger retry.
        s.last_fled = true 
    end
end


function MudCombat.check_summon_success(rid)
    if not MudUtils.check_run(rid) then return end
    local s = MudCombat.state
    if not s.summon_active then return end
    
    if s.last_fled then
        s.last_fled = false
        -- Fled, so retry
        s.summon_retries = 0 
        -- Immediate retry or small delay? 
        -- If we use safe_timer here, we need to make sure test waits for it.
        MudUtils.safe_timer(s.opts.retry_delay, function(rid)
             MudCombat.do_summon(rid)
        end)
    else
        MudCombat.finish_summon(true)
    end
end


function MudCombat.finish_summon(success)
    local s = MudCombat.state
    s.summon_active = false
    local cbs = s.summon_callbacks[s.summon_cb_id]
    if cbs then
        if success and cbs.success then cbs.success() end
        if not success and cbs.fail then cbs.fail() end
    end
    s.summon_callbacks[s.summon_cb_id] = nil
end

return MudCombat
