-- MudCombat Module
-- 戰鬥偵測與召喚管理
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

-- Initialize summon state
MudCombat.summon_state = {
    active = false,
    attempt_id = 0,   -- 每次 summon/retry 遞增，讓舊 timer 自動失效
}

function MudCombat.reset()
    MudCombat.state.last_combat_time = 0
    MudCombat.state.in_combat = false
    
    MudCombat.summon_state = {
        active = false,
        attempt_id = 0,
    }
end

function MudCombat.safe_summon(target_name, summon_cmd, options, on_success, on_fail)
    local s = MudCombat.summon_state
    
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
    s.pending_verification = false
    s.attempt_id = s.attempt_id + 1  -- 遞增，使舊 timer 失效

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
    
    -- 遞增 attempt_id，讓所有舊的 pending timer 自動失效
    s.attempt_id = s.attempt_id + 1
    s.pending_verification = false
    
    s.retries = s.retries + 1
    if s.retries >= s.max_retries then
        s.active = false
        if s.on_fail then s.on_fail() end
        return
    end

    -- 使用 MudUtils.safe_timer（含 run_id 保護）
    local aid = s.attempt_id
    MudUtils.safe_timer(s.retry_delay, function()
        if MudCombat.summon_state.attempt_id ~= aid then return end
        MudCombat.do_summon()
    end)
end

function MudCombat.on_server_message(line)
    local now = os.time()
    local detected = false
    
    -- 1. Check combat keywords
    for _, keyword in ipairs(COMBAT_KEYWORDS) do
        if string.find(line, keyword) then
            detected = true
            break
        end
    end
    
    if detected then
        MudCombat.state.last_combat_time = now
        MudCombat.state.in_combat = true
        if MudCombat.config.debug then mud.echo("[MudCombat] Activity Detected") end
        return true
    end
    
    -- 2. Check summon outcomes if active
    local s = MudCombat.summon_state
    if s.active then
        -- Success: "Papa 突然出現在你的眼前"
        if string.find(line, s.target_name .. ".*突然出現在你的眼前") then
            s.pending_verification = true
            local aid = s.attempt_id
            MudUtils.safe_timer(s.verify_delay, function()
                if MudCombat.summon_state.attempt_id ~= aid then return end
                MudCombat.verify_summon_success()
            end)
            return true
        end
        
        -- Failure: "你失敗了"
        if string.find(line, "你失敗了") then
            s.pending_verification = false 
            MudCombat.retry_summon()
            return true
        end
        
        -- Flee: "Papa 往北邊離開了"
        if string.find(line, s.target_name .. ".*往.*離開了") then
            if s.pending_verification then
                if MudCombat.config.debug then mud.echo("[MudCombat] Target fled during verification!") end
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
    if not s.active then return end
    if not s.pending_verification then return end
    
    -- Success confirmed
    s.active = false
    s.pending_verification = false
    if s.on_success then s.on_success() end
end

function MudCombat.is_fighting()
    local now = os.time()
    return (now - MudCombat.state.last_combat_time) < MudCombat.config.timeout
end

-- Force update combat time
function MudCombat.active()
    MudCombat.state.last_combat_time = os.time()
    MudCombat.state.in_combat = true
end

-- Export to global for timer callbacks
_G.MudCombat = MudCombat

-- ===== 註冊到 Hook Registry =====
local MudUtils = _G.MudUtils
if MudUtils and MudUtils.register_hook then
    MudUtils.register_hook("MudCombat", function(line, clean_line)
        MudCombat.on_server_message(clean_line or line)
    end)
end

return MudCombat
