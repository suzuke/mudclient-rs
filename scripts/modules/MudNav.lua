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

MudNav.config = {
    refresh_cmd = "c ref",
    walk_delay = 0.5,
    debug = true -- Disable debug
}
MudNav.state = {
    walking = false,
    paused = false,
    queue = {},
    index = 1,
    callback = nil,
    recording = false,
    recorded_path = {}
}

function MudNav.reset()
    local s = MudNav.state
    s.walking = false
    s.paused = false
    s.queue = {}
    s.index = 1
    s.callback = nil
    s.last_send_time = nil
    s.waiting_confirm = false
end

function MudNav.walk(path, callback)
    MudNav.reset()
    local s = MudNav.state
    s.walking = true
    s.paused = false
    
    -- Handle both string paths and table paths
    if type(path) == "string" then
        s.queue = MudUtils.parse_cmds(path)
    elseif type(path) == "table" then
        s.queue = path
    else
        mud.echo("[MudNav] Error: Invalid path type")
        s.walking = false
        return
    end
    
    s.index = 1
    s.callback = callback
    if MudNav.config.debug then mud.echo("[MudNav] Start Walk (" .. #s.queue .. " steps)") end
    mud.emit("nav_walk_started", {steps = #s.queue})
    MudNav.send_next()
end

function MudNav.send_next()
    local s = MudNav.state
    if not s.walking or s.paused then return end
    
    if s.index > #s.queue then
        if MudNav.config.debug then mud.echo("[MudNav] Walk Complete.") end
        s.walking = false
        mud.emit("nav_walk_completed", {steps = #s.queue})
        if s.callback then s.callback(true) end -- true = success
        return
    end
    
    -- 防洪鎖：如果上一步指令還在處理中或剛發送不久
    local now = os.clock()
    if s.last_send_time and (now - s.last_send_time < 0.1) then 
        if MudNav.config.debug then mud.echo("[MudNav] Throttled (" .. (now - s.last_send_time) .. "s). Rescheduling.") end
        MudUtils.safe_timer(0.1, function() MudNav.send_next() end)
        return 
    end
    
    local step = s.queue[s.index]
    local cmd = ""
    local expected_id = nil
    
    if type(step) == "table" then
        cmd = step.cmd
        expected_id = step.id
    else
         cmd = step
    end
    
    if MudNav.config.debug then mud.echo("[MudNav] Sending[" .. s.index .. "]: " .. cmd) end
    
    s.last_send_time = now
    s.waiting_confirm = true -- 等待伺服器回應（出口或特定訊息）
    s.current_step_obj = step -- Store for verification
    s.forced_look = false -- 重置 auto-look 標記
    
    -- [NEW] Auto-Look Timeout (Recover if no room update, e.g. recall at start)
    if s.confirm_timer_id then 
        -- Cancel previous (though should be cleared by confirm)
        -- MudUtils.cancel_timer(s.confirm_timer_id) -- Not exposed yet?
        -- Just overwrite ID. If previous runs, it checks waiting_confirm anyway.
    end
    
    local timer_step = s.index -- 記錄此計時器屬於哪一步
    s.confirm_timer_id = MudUtils.safe_timer(3.0, function()
        if s.walking and not s.paused and s.waiting_confirm and s.index == timer_step then
             if MudNav.config.debug then mud.echo("[MudNav] Step Stuck (No Exits). Forcing Look...") end
             s.forced_look = true -- 標記為 auto-look 觸發
             mud.send("l")
        end
    end)
    
    if mud then mud.send(cmd) end
end

-- ===== Path Recording =====
function MudNav.record_start()
    MudNav.state.recording = true
    MudNav.state.recorded_path = {}
    mud.echo("[MudNav] 🔴 開始錄製路徑 (請開始移動)")
end

function MudNav.record_stop()
    if not MudNav.state.recording then
        mud.echo("[MudNav] ⚠️ 目前沒有在錄製")
        return
    end
    MudNav.state.recording = false
    mud.echo("[MudNav] ⏹️ 錄製結束。路徑代碼如下：")
    mud.echo("---------------------------------------------------")
    
    -- Generate Lua Code
    local lines = {}
    table.insert(lines, "local path = {")
    for _, step in ipairs(MudNav.state.recorded_path) do
        table.insert(lines, string.format('    {cmd="%s", id="%s"},', step.cmd, step.id))
    end
    table.insert(lines, "}")
    
    for _, l in ipairs(lines) do
         mud.echo(l)
    end
    mud.echo("---------------------------------------------------")
    -- Copy to clipboard hint? 
end

-- Helper to capture user movement for recording
-- We need to hook user input or rely on successful moves
-- Better reliability: Hook `send`? Or just listen to "Exits" and assume previous command?
-- Complex. 
-- Best approach: When "Exits" seen (move successful), record the command that caused it??
-- But we don't know the command easily if typed by user.
-- Alternative: User types `/lua MudNav.rec("n")`? No, too tedious.
-- Function to be called BY ALIASES. e.g. alias n = lua MudNav.record_step("n"); n
-- Or, we use the Room ID change.
-- Store `last_room_id`.
-- When Room ID changes, we try to deduce direction? Hard.
-- Let's provide a helper `MudNav.move(dir)` for recording.
function MudNav.move(dir)
    if MudNav.state.recording then
        mud.send(dir)
        -- We tentatively record it, but we should confirm ID...
        -- Let's just push it to a pending state.
        MudNav.state.pending_record_cmd = dir
    else
        mud.send(dir)
    end
end

-- But user wants to just walk.
-- If we can't hook input, maybe we just use `on_server_message` to detect "Exits" and then
-- look at `mud.last_sent_command`? Not available.
-- Let's defer strict recording for now, and implement standard verification first.
-- Wait, I promised "Automatic Path Recording".
-- "You just walk as usual".
-- This implies I need to know what they typed.
-- `mud` object might expose `on_input` hook? 
-- If not, I'll assume they use mapping or aliases I provide?
-- Or I just assume I can't track *what* they typed unless they use an alias.
-- Let's implement a simple "Check ID on Arrival" first.
-- And for recording, I will check `mud.get_current_room_id()` periodically?
-- NO. I'll stick to the "Verification" implementation first.
-- For recording, I'll add a simple `MudNav.add_step(dir)`
-- user can make aliases: `alias n = lua MudNav.add_step("n")`

function MudNav.on_server_message(line, clean_line)
    local s = MudNav.state
    -- Use clean_line for matching if available, fallback to line
    local text = clean_line or line
    
    -- System Hooks for Recording
    if s.recording then
        -- 1. Capture User Input (Echoed as "> cmd")
        local cmd_input = text:match("^> (.*)")
        if not cmd_input then
             cmd_input = text:match("^>(.*)")
        end
        
        if cmd_input then
            local clean_cmd = cmd_input:match("^%s*(.-)%s*$")
            if clean_cmd ~= "" then
                s.last_rec_cmd = clean_cmd
            end
            return
        end
        
        -- 2. Capture action success (open, unlock, etc. do not change room ID)
        if s.last_rec_cmd then
            local cmd = string.lower(s.last_rec_cmd)
            if cmd:match("^open") or cmd:match("^op") or 
               cmd:match("^unlock") or cmd:match("^un") or 
               cmd:match("^enter") or cmd:match("^ent") or
               cmd:match("^push") or cmd:match("^look") or cmd:match("^l%s") or 
               cmd:match("^ta%s") or cmd:match("^talk%s") then
               
               local success_patterns = {"Ok%.", "opened", "OK%.", "打開了", "解開了", "已經打開", "門老早就是開著", "光芒閃起", "時空突然扭曲", "漸漸的消失了"}
               for _, pat in ipairs(success_patterns) do
                   if string.find(text, pat) then
                       local current_id = mud and mud.get_current_room_id() or "unknown"
                       table.insert(s.recorded_path, { cmd = s.last_rec_cmd, id = current_id })
                       mud.echo("[MudNav] ⏺️ 動作記錄成功: " .. s.last_rec_cmd)
                       s.last_rec_cmd = nil
                       return
                   end
               end
            end
        end

        -- 3. Capture Room ID (Confirming the move)
        local id = text:match("%(ID: (.-)%)")
        if id then
             if s.last_rec_cmd then
                 table.insert(s.recorded_path, { cmd = s.last_rec_cmd, id = id })
                 mud.echo("[MudNav] ⏺️ 記錄成功: " .. s.last_rec_cmd .. " -> " .. string.sub(id, 1, 8))
                 s.last_rec_cmd = nil
             end
        end
        
        return 
    end

    if not s.walking then return end
    
    -- 偵測碰撞/失敗 (重置等待狀態)
    if string.find(text, "這個方向沒有路") or string.find(text, "不能往") then
        s.waiting_confirm = false
        -- 撞牆時通常應該停止或跳過，這裡選擇 advance
        MudUtils.safe_timer(MudNav.config.walk_delay, function()
            if not s.walking or s.paused then return end
            s.index = s.index + 1
            MudNav.send_next()
        end)
        return
    end

    -- Detect 移動力
    if string.find(text, "精疲力竭") or string.find(text, "移動力不足") then
        if not s.paused then
            s.paused = true
            s.waiting_confirm = false
            mud.emit("nav_stamina_exhausted", {step = s.index, total = #s.queue})
            if mud then mud.send(MudNav.config.refresh_cmd) end
        end
        return
    end
    
    if string.find(text, "recovering") or string.find(text, "恢復") then
        if s.paused then
            s.paused = false
            MudUtils.safe_timer(0.5, function() MudNav.send_next() end)
        end
        return
    end

    -- Detect Exits (核心前進與驗證邏輯)
    if string.find(text, "%[出口:") then
        if MudNav.config.debug then mud.echo("[MudNav] Detected Exits. Waiting=" .. tostring(s.waiting_confirm)) end
        
        -- 防重複觸發
        local now = os.clock()
        if s.last_exit_process_time and (now - s.last_exit_process_time < 0.1) then
            if MudNav.config.debug then mud.echo("[MudNav] Ignoring duplicate exit signal") end
            return
        end
        s.last_exit_process_time = now
        
        -- 核心防洪邏輯：只有在我們確實在等待確認時才前進
        if s.waiting_confirm and not s.paused then
             s.waiting_confirm = false
             
             -- ID 驗證邏輯：直接用 API 查詢 Rust 端已計算好的 Room ID
             local step = s.current_step_obj
             if type(step) == "table" and step.id then
                 local actual_id = mud and mud.get_current_room_id() or nil
                 if actual_id and actual_id == step.id then
                     if MudNav.config.debug then mud.echo("[MudNav] ✅ ID 驗證通過: " .. string.sub(actual_id, 1, 8)) end
                 elseif actual_id then
                     mud.echo("🛑 [MudNav] 導航錯誤：路徑偏移！")
                     mud.echo("   預期 ID: " .. tostring(step.id))
                     mud.echo("   實際 ID: " .. tostring(actual_id))
                     s.walking = false
                     mud.emit("nav_walk_failed", {reason = "id_mismatch", expected = step.id, actual = actual_id})
                     if s.callback then s.callback(false, "id_mismatch") end
                     return
                 else
                     if MudNav.config.debug then mud.echo("[MudNav] ⚠️ 未收到 Room ID，跳過驗證") end
                 end
             end
             
             -- auto-look 後用較長延遲，避免下一步指令太快被吃掉
             local delay = s.forced_look and 1.5 or MudNav.config.walk_delay
             s.forced_look = false
             MudUtils.safe_timer(delay, function()
                 if MudNav.config.debug then mud.echo("[MudNav] Timer Fired. Index=" .. s.index) end
                 if not s.walking or s.paused then return end
                 s.index = s.index + 1
                 MudNav.send_next()
             end)
        end
        return
    end

    
    -- Detect Closed/Locked Doors
    if string.find(text, "關著") or string.find(text, "鎖著") then
        s.waiting_confirm = false
        
        -- Handle both string and table cmd
        local raw_cmd = s.queue[s.index]
        local cmd = (type(raw_cmd) == "table") and raw_cmd.cmd or raw_cmd
        
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
    local raw_cmd = s.queue[s.index]
    local current_cmd = (type(raw_cmd) == "table") and raw_cmd.cmd or raw_cmd
    
    if current_cmd and (current_cmd:match("^open") or current_cmd:match("^op") or 
                        current_cmd:match("^unlock") or current_cmd:match("^un") or 
                        current_cmd:match("^enter") or current_cmd:match("^ent") or
                        current_cmd:match("^push") or current_cmd:match("^look") or current_cmd:match("^l%s") or
                        current_cmd:match("^ta%s") or current_cmd:match("^talk%s")) then
                        
         local success_patterns = {"Ok%.", "opened", "OK%.", "打開了", "解開了", "已經打開", "門老早就是開著", "光芒閃起", "時空突然扭曲", "漸漸的消失了"}
         
         for _, pat in ipairs(success_patterns) do
             if string.find(text, pat) then
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

-- Hook Helper for Recorder
-- This allows user to manually trigger a record step if they bind aliases
function MudNav.record_step(cmd)
    if not MudNav.state.recording then 
        mud.send(cmd)
        return 
    end
    
    -- Assume we are at Start Room A. User types 'n'.
    -- We send 'n'.
    -- Next prompt/room event -> We act. (Async)
    -- Simplified: Check ID *after* move? 
    -- Let's just store the step with a placeholder, and fill ID later?
    -- No, simpler: Trigger the move, wait for delay, then capture ID.
    
    local old_id = mud.get_current_room_id()
    mud.send(cmd)
    
    -- Delayed check
    MudUtils.safe_timer(1.0, function()
        local new_id = mud.get_current_room_id()
        if new_id and new_id ~= old_id then
            table.insert(MudNav.state.recorded_path, {cmd=cmd, id=new_id})
            mud.echo("[Rec] " .. cmd .. " -> " .. string.sub(new_id, 1, 8))
        else
            mud.echo("[Rec] " .. cmd .. " (No ID Change/Fail)")
        end
    end)
end

-- ===== 註冊到 Hook Registry =====
MudUtils.register_hook("MudNav", function(line, clean_line, is_echo)
    if is_echo then return end
    MudNav.on_server_message(line, clean_line)
end)

_G.MudNav = MudNav
return MudNav
