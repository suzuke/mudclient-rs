-- MudNav Module
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
    error("MudNav cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

local MudNav = {}
_G.MudNav = MudNav 

MudNav.config = {
    refresh_cmd = "c ref",
    walk_delay = 0.5,
    debug = false -- Disable debug
}
MudNav.state = {
    walking = false,
    paused = false,
    queue = {},
    index = 1,
    callback = nil
}

function MudNav.reset()
    local s = MudNav.state
    s.walking = false
    s.paused = false
    s.queue = {}
    s.index = 1
    s.callback = nil
end

function MudNav.walk(path, callback)
    MudNav.reset()
    local s = MudNav.state
    s.walking = true
    s.paused = false
    s.queue = MudUtils.parse_cmds(path)
    s.index = 1
    s.callback = callback
    if MudNav.config.debug then mud.echo("[MudNav] Start Walk: " .. path .. " (" .. #s.queue .. " steps)") end
    MudNav.send_next()
end

function MudNav.send_next()
    local s = MudNav.state
    if not s.walking or s.paused then return end
    
    if s.index > #s.queue then
        if MudNav.config.debug then mud.echo("[MudNav] Walk Complete.") end
        s.walking = false
        if s.callback then s.callback() end
        return
    end
    
    -- 防洪鎖：如果上一步指令還在處理中或剛發送不久
    local now = os.clock()
    if s.last_send_time and (now - s.last_send_time < 0.1) then 
        return 
    end
    
    local cmd = s.queue[s.index]
    if MudNav.config.debug then mud.echo("[MudNav] Sending[" .. s.index .. "]: " .. cmd) end
    
    s.last_send_time = now
    s.waiting_confirm = true -- 等待伺服器回應（出口或特定訊息）
    
    if mud then mud.send(cmd) end
end

function MudNav.on_server_message(line)
    if not MudNav.state.walking then return end
    local s = MudNav.state
    
    -- 偵測碰撞/失敗 (重置等待狀態)
    if string.find(line, "這個方向沒有路") or string.find(line, "不能往") then
        s.waiting_confirm = false
        -- 撞牆時通常應該停止或跳過，這裡選擇 advance
        MudUtils.safe_timer(MudNav.config.walk_delay, function()
            s.index = s.index + 1
            MudNav.send_next()
        end)
        return
    end

    -- Detect Stamina
    if string.find(line, "exhausted") or string.find(line, "精疲力竭") or string.find(line, "移動力不足") then
        s.paused = true
        s.waiting_confirm = false
        if mud then mud.send(MudNav.config.refresh_cmd) end
        return
    end
    
    if string.find(line, "recovering") or string.find(line, "恢復") then
        s.paused = false
        MudUtils.safe_timer(0.5, function() MudNav.send_next() end)
        return
    end

    -- Detect Exits (to advance)
    if string.find(line, "%[Exits:") or string.find(line, "%[出口:") then
        if MudNav.config.debug then mud.echo("[MudNav] Detected Exits. Waiting=" .. tostring(s.waiting_confirm)) end
        
        -- 核心防洪邏輯：只有在我們確實在等待確認時才前進
        if s.waiting_confirm and not s.paused then
             s.waiting_confirm = false -- 已收到回應，清除標記
             MudUtils.safe_timer(MudNav.config.walk_delay, function()
                 if not s.walking or s.paused then return end
                 s.index = s.index + 1
                 MudNav.send_next()
             end)
        end
        return
    end
    
    -- Detect Closed/Locked Doors
    if string.find(line, "closed") or string.find(line, "lock") or string.find(line, "關著") or string.find(line, "鎖著") then
        s.waiting_confirm = false
        local cmd = s.queue[s.index]
        if cmd and mud then 
            mud.send("unlock " .. cmd)
            mud.send("open " .. cmd)
            MudUtils.safe_timer(0.5, function() 
                 if s.walking and not s.paused then
                     s.waiting_confirm = true
                     mud.send(cmd) 
                 end
            end)
        end
        return
    end

    -- [Enhanced] Detect Action Success
    local current_cmd = s.queue[s.index]
    if current_cmd and (current_cmd:match("^open") or current_cmd:match("^op") or 
                        current_cmd:match("^unlock") or current_cmd:match("^un") or 
                        current_cmd:match("^enter") or current_cmd:match("^ent") or
                        current_cmd:match("^push")) then
                        
         local success_patterns = {"Ok%.", "opened", "OK%.", "打開了", "解開了", "已經打開", "門老早就是開著"}
         
         for _, pat in ipairs(success_patterns) do
             if string.find(line, pat) then
                 if s.waiting_confirm and not s.paused then
                     s.waiting_confirm = false
                     MudUtils.safe_timer(MudNav.config.walk_delay, function()
                         if s.walking and not s.paused then
                             s.index = s.index + 1
                             MudNav.send_next()
                         end
                     end)
                 end
                 return
             end
         end
    end
end

return MudNav
