-- MudCombat Module
local MudCombat = {}
MudCombat.__index = MudCombat

MudCombat.state = {
    in_combat = false,
    last_combat_time = 0,
}

MudCombat.config = {
    timeout = 3, -- Seconds to wait before considering combat ended
    debug = false,
}

-- Key phrases that indicate active combat
local COMBAT_KEYWORDS = {
    "口吐鮮血", "皮綻肉開", "生死的邊緣掙扎", 
    "攻擊", "閃躲", "不需要.*休息",
    "正在這兒攻擊著 你!", "伺機而動", "蓄勢待發", "盤算著對手",
    "喝道", "叫道", "嚷道", "這一擊", "造成了", "遭受了",
    "迴避", "格擋", "招架", "慘叫", "倒地", "逃跑",
    "只剩.*氣", "搖搖欲墜", "已經面無血色了",
    "絲毫不放在眼裡", "用力一擊", "身陷戰鬥中"
}


function MudCombat.reset()
    MudCombat.state.last_combat_time = 0
    MudCombat.state.in_combat = false
    
    -- Reset summon state
    MudCombat.summon_state = {
        active = false,
        timer_id = nil,
    }
end

-- Initialize summon state
MudCombat.summon_state = {
    active = false,
    timer_id = nil,
}

local function require_mudutils()
    -- Try to load MudUtils if available, for timers
    if _G.MudUtils then return _G.MudUtils end
    local status, mod = pcall(require, "scripts.modules.MudUtils")
    if status then return mod end
    status, mod = pcall(require, "modules.MudUtils")
    if status then return mod end
    return nil
end


function MudCombat.safe_summon(target_name, summon_cmd, options, on_success, on_fail)
    local s = MudCombat.summon_state
    
    -- Validations
    if not target_name or not summon_cmd then
        if on_fail then on_fail() end
        return
    end

    -- Setup state
    s.active = true
    s.target_name = target_name
    s.summon_cmd = summon_cmd
    s.on_success = on_success
    s.on_fail = on_fail
    
    -- Options defaults
    options = options or {}
    s.max_retries = options.max_retries or 3
    s.retry_delay = options.retry_delay or 2.0
    s.verify_delay = options.verify_delay or 1.0
    s.retries = 0
    s.verify_timer_id = nil -- Reset verify timer

    MudCombat.do_summon()
end

function MudCombat.do_summon()
    local s = MudCombat.summon_state
    if not s.active then return end
    
    mud.send(s.summon_cmd)
end

function MudCombat.retry_summon()
    local s = MudCombat.summon_state
    if not s.active then return end
    
    -- Cancel any pending verification if we are retrying
    -- (e.g. noticed flee after success msg)
    if s.verify_timer_id then
        -- We can't easily cancel in some engines without ID, but we can invalidate the state
        -- by setting verify_timer_id to nil, and checking it in verify callback?
        -- Actually, the callback checks s.active. 
        -- But s.active remains true during retry! 
        -- So we need a specific way to tell the pending callback "you are obsolete".
        -- Let's use a unique generation ID or simply clear verify_timer_id and check it in callback?
        -- No, the callback doesn't know *which* timer ID it was.
        -- Better: increment a 'summon_attempt_id' every time we retry/start.
        s.verify_timer_id = nil
    end
    
    s.retries = s.retries + 1
    if s.retries >= s.max_retries then
        s.active = false
        if s.on_fail then s.on_fail() end
        return
    end

    if mud and mud.timer then
        mud.timer(s.retry_delay, "_G.MudCombat.do_summon()")
    end
end

function MudCombat.on_server_message(line)
    local now = os.time()
    local detected = false
    
    -- 1. Check existing combat keywords
    for _, keyword in ipairs(COMBAT_KEYWORDS) do
        if string.find(line, keyword) then
            detected = true
            break
        end
    end
    
    if detected then
        MudCombat.state.last_combat_time = now
        MudCombat.state.in_combat = true
        if MudCombat.config.debug then print("[MudCombat] Activity Detected") end
        return true
    end
    
    -- 2. Check summon outcomes if active
    local s = MudCombat.summon_state
    if s.active then
        -- Success pattern: "Papa 突然出現在你的眼前"
        if string.find(line, s.target_name .. ".*突然出現在你的眼前") then
            -- Verify delay before success callback (to check for immediate flee)
            if mud and mud.timer then
                -- Store the timestamp or ID to validate in callback
                s.last_success_time = os.time()
                s.pending_verification = true
                s.verify_timer_id = mud.timer(s.verify_delay, "_G.MudCombat.verify_summon_success()")
            end
            return true
        end
        
        -- Failure pattern: "你失敗了"
        if string.find(line, "你失敗了") then
            s.pending_verification = false 
            MudCombat.retry_summon()
            return true
        end
        
        -- Flee pattern (checking if target leaves immediately)
        -- "Papa 往北邊離開了"
        if string.find(line, s.target_name .. ".*往.*離開了") then
            -- If verification is pending, we MUST cancel it effectively
            if s.pending_verification then
                if MudCombat.config.debug then print("[MudCombat] Target fled during verification!") end
                s.pending_verification = false 
                MudCombat.retry_summon()
                return true
            end
        end
    end
    
    return false
end

function MudCombat.verify_summon_success()
    local s = MudCombat.summon_state
    -- If active is false, do nothing
    if not s.active then return end
    
    -- Check if we are still pending verification
    if not s.pending_verification then
        -- This means a flee or failure happened in the meantime
        return 
    end
    
    -- Success confirmed
    s.active = false
    s.pending_verification = false
    if s.on_success then s.on_success() end
end

function MudCombat.is_fighting()
    -- check if we are within timeout window
    local now = os.time()
    if now - MudCombat.state.last_combat_time < MudCombat.config.timeout then
        return true
    end
    return false
end

-- Force update combat time (e.g. when "You are fighting!" message is received)
function MudCombat.active()
    MudCombat.state.last_combat_time = os.time()
    MudCombat.state.in_combat = true
end

-- Export to global for timer callbacks
_G.MudCombat = MudCombat

return MudCombat
