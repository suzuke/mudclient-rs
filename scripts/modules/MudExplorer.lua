-- MudExplorer Module
local MudExplorer = {}
MudExplorer.__index = MudExplorer

function MudExplorer.new()
    local self = setmetatable({}, MudExplorer)
    self.path_stack = {}
    self.visited = {}      -- { "x,y,z" = true }
    self.visited_ids = {}  -- { "hash" = true } [NEW]
    self.is_done = false
    -- coordinates? pos = {x=0, y=0, z=0}
    return self
end

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
    error("MudExplorer cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

-- DFS Logic
-- Direction mapping
local DIR_INFO = {
    {name="北", cmd="n", dx=0, dy=1, dz=0},
    {name="南", cmd="s", dx=0, dy=-1, dz=0},
    {name="東", cmd="e", dx=1, dy=0, dz=0},
    {name="西", cmd="w", dx=-1, dy=0, dz=0},
    {name="上", cmd="u", dx=0, dy=0, dz=1},
    {name="下", cmd="d", dx=0, dy=0, dz=-1},
}
local DIR_BY_NAME = {}
local DIR_BY_CMD = {}
for _, d in ipairs(DIR_INFO) do
    DIR_BY_NAME[d.name] = d
    DIR_BY_CMD[d.cmd] = d
    -- Add English names for testing/compatibility
    if d.name == "北" then DIR_BY_NAME["north"] = d end
    if d.name == "南" then DIR_BY_NAME["south"] = d end
    if d.name == "東" then DIR_BY_NAME["east"] = d end
    if d.name == "西" then DIR_BY_NAME["west"] = d end
    if d.name == "上" then DIR_BY_NAME["up"] = d end
    if d.name == "下" then DIR_BY_NAME["down"] = d end
end

local REVERSE_CMD = {n="s", s="n", e="w", w="e", u="d", d="u"}
local DIR_PRIORITY = {"北", "東", "南", "西", "上", "下", "north", "east", "south", "west", "up", "down"}

local function pos_key(pos)
    return pos.x .. "," .. pos.y .. "," .. pos.z
end

local function parse_exits(line)
    local exits = {}
    -- Support both [出口: 北 南] and [Exits: north south]
    local content = string.match(line, "%[出口:%s*(.-)%]") or string.match(line, "%[Exits:%s*(.-)%]")
    if content then
        for dir in string.gmatch(content, "%S+") do
            if DIR_BY_NAME[dir] then
                table.insert(exits, dir)
            end
        end
    end
    return exits
end

function MudExplorer:start()
    self.is_done = false
    self.visited = {}
    self.visited_ids = {} -- [NEW]
    self.path_stack = {}
    self.pos = {x=0, y=0, z=0}
    self.pending_move = nil
    self.room_count = 0
end

function MudExplorer:confirm_move()
    if self.pending_move then
        if self.pending_move.type == "forward" then
            local d = self.pending_move.d
            self.pos = {x=self.pos.x+d.dx, y=self.pos.y+d.dy, z=self.pos.z+d.dz}
            table.insert(self.path_stack, {cmd=d.cmd, rev=REVERSE_CMD[d.cmd]})
        elseif self.pending_move.type == "backtrack" then
            local rev = self.pending_move.rev_cmd
            local d_back = DIR_BY_CMD[rev]
            if d_back then
                self.pos = {x=self.pos.x+d_back.dx, y=self.pos.y+d_back.dy, z=self.pos.z+d_back.dz}
            end
            if #self.path_stack > 0 then table.remove(self.path_stack) end
        end
        self.pending_move = nil
    end
end

function MudExplorer:process_room(line)
    -- 1. Parse exits
    local exits = parse_exits(line)
    if not exits or #exits == 0 then
        -- In case of parsing error or no exits, we still need to process
        -- But really we assume line contains exits. 
    end
    
    -- 2. Confirm pending move (update pos)
    self:confirm_move()
    
    -- [NEW] Room ID Check
    local room_id = mud and mud.get_current_room_id() or nil
    
    -- 3. Mark visited (Multi-criteria)
    local p_key = pos_key(self.pos)
    local is_new_pos = not self.visited[p_key]
    
    -- Logic: If ID is seen before, treat as visited even if pos is new (Loop closed)
    local already_visited = false
    
    if room_id then
        if self.visited_ids[room_id] then 
            already_visited = true 
            -- Mark current pos as visited too
            self.visited[p_key] = true
        else
            self.visited_ids[room_id] = true
        end
    end
    
    if is_new_pos then
        self.visited[p_key] = true
        self.room_count = self.room_count + 1
    else
        already_visited = true
    end

    if MudExplorer.config.debug then
        mud.echo(string.format("[MudExplorer] Pos:%s, ID:%s, NewPos:%s, VisitedBefore:%s", 
            p_key, tostring(room_id), tostring(is_new_pos), tostring(already_visited)))
    end

    if already_visited then
        if MudExplorer.config.debug then mud.echo("[MudExplorer] Loop detected (ID/Pos known). Backtracking.") end
    end

    -- 4. Decide next move (嘗試未訪問的鄰居 — 不論是否已訪問過當前位置)
    for _, dir_name in ipairs(DIR_PRIORITY) do
        local d = DIR_BY_NAME[dir_name]
        if d then
            local has_exit = false
            for _, ex in ipairs(exits) do
                if ex == dir_name or ex == d.cmd or ex == d.name then has_exit = true; break end
            end
    
            if has_exit then
                local next_pos = {x=self.pos.x+d.dx, y=self.pos.y+d.dy, z=self.pos.z+d.dz}
                local next_key = pos_key(next_pos)
                
                if not self.visited[next_key] then
                    self.pending_move = {type="forward", d=d}
                    return d.cmd
                end
            end
        end
    end

    -- 5. Backtrack
    if #self.path_stack > 0 then
        local last = self.path_stack[#self.path_stack]
        self.pending_move = {type="backtrack", rev_cmd=last.rev}
        return last.rev
    end

    self.is_done = true
    return nil
end

-- ============================================================
-- High-Level Automation
-- ============================================================

MudExplorer.config = {
    target = nil,
    max_laps = 5,
    debug = false,
    disable_open_doors = false,
}

MudExplorer.state = {
    exploring = false,
    instance = nil,
    callback = nil,
    laps = 0,
    check_timer = nil,
    last_exit_line = nil,
    doors_opened = false,
    target_in_room = false
}

function MudExplorer.explore(callback)
    local s = MudExplorer.state
    s.exploring = true
    s.callback = callback
    s.laps = 0
    s.instance = MudExplorer.new() -- Create new DFS instance
    s.instance:start()
    s.target_in_room = false
    s.doors_opened = false
    
    if MudExplorer.config.debug then mud.echo("[MudExplorer] Start Explore") end
    
    -- Initial Look (Don't open blindly, wait for Exits)
    mud.send("l")
end

-- Open doors only for directions NOT in visible exits
function MudExplorer.try_open_doors(visible_exits)
    if MudExplorer.config.disable_open_doors then return end

    local exit_set = {}
    if visible_exits then
        for _, dir in ipairs(visible_exits) do
            exit_set[dir] = true
        end
    end

    local commands = {}
    -- Only try to open directions that are NOT visible exits (potential hidden doors)
    for _, dir_info in ipairs(DIR_INFO) do
        if not exit_set[dir_info.name] then
             table.insert(commands, "op " .. dir_info.cmd)
        end
    end
    
    -- Send commands
    if #commands > 0 then
        mud.send(table.concat(commands, ";"))
    end
end

function MudExplorer.stop()
    MudExplorer.state.exploring = false
    if MudExplorer.config.debug then mud.echo("[MudExplorer] Stopped") end
end

function MudExplorer.status()
    local s = MudExplorer.state
    if s.exploring and s.instance then
        mud.echo("   DFS Nodes: " .. s.instance.room_count)
        mud.echo("   DFS Depth: " .. #s.instance.path_stack)
        mud.echo("   Laps: " .. s.laps .. "/" .. MudExplorer.config.max_laps)
    else
        mud.echo("   Not exploring.")
    end
end

-- Event Handler
function MudExplorer.on_server_message(line)
    local s = MudExplorer.state
    if not s.exploring then return end
    
    -- 1. Check Target
    if MudExplorer.config.target and string.find(string.lower(line), string.lower(MudExplorer.config.target), 1, true) then
        if MudExplorer.config.debug then mud.echo("[MudExplorer] Target Found: " .. line) end
        
        -- FIX: Confirm the pending move that brought us here, effectively updating the stack
        if s.instance then s.instance:confirm_move() end
        
        s.exploring = false
        if s.callback then s.callback(true, line) end
        return
    end

    -- 2. Check Exits (Trigger Move)
    if string.find(line, "%[Exits:") or string.find(line, "%[出口:") then
        -- Double-hook protection: If we just processed this line (or same room type/exits), ignore
        local now = os.clock()
        if s.last_exit_process_time and (now - s.last_exit_process_time < 0.1) then
            if MudExplorer.config.debug then mud.echo("[MudExplorer] Ignoring duplicate exit signal") end
            return
        end
        s.last_exit_process_time = now

        -- Delay processing to ensure all room content (mobs) is seen
        s.last_exit_line = line
        MudUtils.safe_timer(0.5, MudExplorer.process_step_dispatch) 
    end
    
    -- 3. Check Doors
    if string.find(line, "門是關著的") or string.find(line, "The door is closed") then
        local inst = s.instance
        if inst and inst.pending_move then
            local cmd = inst.pending_move.type == "forward" and inst.pending_move.d.cmd or inst.pending_move.rev_cmd
            mud.send("op " .. cmd)
            MudUtils.safe_timer(1.0, function() mud.send(cmd) end)
        end
    end
    
    -- 4. Stamina
    if string.find(line, "你精疲力竭了") or string.find(line, "你的移動力不足") then
         mud.send("c ref")
         -- Retry last move? 
         -- If we are stuck, we need to retry.
         -- Simple retry logic:
         MudUtils.safe_timer(3.0, function() 
             local inst = s.instance
             if inst and inst.pending_move then
                 local cmd = inst.pending_move.type == "forward" and inst.pending_move.d.cmd or inst.pending_move.rev_cmd
                 mud.send(cmd)
             end
         end)
    end
end

function MudExplorer.process_step_dispatch(rid)
    if not MudUtils.check_run(rid) then return end
    local s = MudExplorer.state
    -- Strict check: if exploring is false (e.g. target found), stop immediately
    if not s.exploring then return end

    -- If we just arrived and haven't opened doors (to find hidden ones), try opening
    local inst = s.instance
    if inst.pending_move and inst.pending_move.type == "forward" and not s.doors_opened and not MudExplorer.config.disable_open_doors then
        s.doors_opened = true
        
        -- Parse current exits to avoid redundant opening
        local exits = parse_exits(s.last_exit_line or "")
        MudExplorer.try_open_doors(exits)
        
        -- After trying to open, we should look again to update exits?
        mud.send("l")
        return
    end
    
    s.doors_opened = false
    MudExplorer.process_next_step()
end

function MudExplorer.process_next_step()
    local s = MudExplorer.state
    local line = s.last_exit_line or ""
    local next_cmd = s.instance:process_room(line)
    
    if next_cmd then
        if MudExplorer.config.debug then mud.echo("[MudExplorer] Move: " .. next_cmd) end
        mud.send(next_cmd)
    else
        -- Done with this lap
        s.laps = s.laps + 1
        if s.laps >= MudExplorer.config.max_laps then
            if MudExplorer.config.debug then mud.echo("[MudExplorer] Max Laps Reached") end
            s.exploring = false
            if s.callback then s.callback(false, nil) end
        else
            if MudExplorer.config.debug then mud.echo("[MudExplorer] Starting Lap " .. (s.laps + 1)) end
            -- Restart
            s.instance:start()
            
            -- Keep start node visited??
            -- Logic: If we reset visited, we might cycle immediately if stuck?
            -- But we want to re-explore potentially.
            s.instance.visited_ids = {}
            s.instance.visited = {}
            s.instance.visited[pos_key(s.instance.pos)] = true
             -- Also mark start ID if available (needs query? will happen in first process_room call)
            
            MudExplorer.process_next_step() -- Recursively call to find next move from start
        end
    end
end


function MudExplorer.resume(callback)
    local s = MudExplorer.state
    if not s.instance then
        if MudExplorer.config.debug then mud.echo("[MudExplorer] Cannot resume: No instance") end
        return
    end

    s.exploring = true
    s.callback = callback -- Update callback if needed
    s.target_in_room = false
    s.doors_opened = false
    
    if MudExplorer.config.debug then mud.echo("[MudExplorer] Resuming Explore") end
    
    -- Immediately process next step based on current state
    -- We need to act as if we just arrived in the room
    mud.send("l")
end

function MudExplorer.get_path_to_start()
    local s = MudExplorer.state
    if not s.instance then return nil end
    
    local path = {}
    -- Reverse iteration of path_stack
    for i = #s.instance.path_stack, 1, -1 do
        table.insert(path, s.instance.path_stack[i].rev)
    end
    
    return table.concat(path, ";")
end

-- ===== 註冊到 Hook Registry =====
MudUtils.register_hook("MudExplorer", function(line, clean_line)
    MudExplorer.on_server_message(clean_line or line)
end)

return MudExplorer
