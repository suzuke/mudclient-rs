-- MudExplorer Module
local MudExplorer = {}
MudExplorer.__index = MudExplorer

function MudExplorer.new()
    local self = setmetatable({}, MudExplorer)
    self.path_stack = {}
    self.visited = {}
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
    self.path_stack = {}
    self.pos = {x=0, y=0, z=0}
    self.pending_move = nil
    self.room_count = 0
end

function MudExplorer:process_room(line)
    -- 1. Parse exits
    local exits = parse_exits(line)
    if not exits or #exits == 0 then
        -- In case of parsing error or no exits, we still need to process
        -- But really we assume line contains exits. 
        -- If line doesn't contain exits, maybe we shouldn't return anything yet?
        -- For the sake of the module, we assume this function is called when exits are found.
    end

    -- 2. Confirm pending move (update pos)
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

    -- 3. Mark visited
    local key = pos_key(self.pos)
    if not self.visited[key] then
        self.visited[key] = true
        self.room_count = self.room_count + 1
    end

    -- 4. Decide next move
    -- Try unvisited neighbors
    for _, dir_name in ipairs(DIR_PRIORITY) do
        local has_exit = false
        for _, ex in ipairs(exits) do
            if ex == dir_name then has_exit = true; break end
        end

        if has_exit then
            local d = DIR_BY_NAME[dir_name]
            local next_pos = {x=self.pos.x+d.dx, y=self.pos.y+d.dy, z=self.pos.z+d.dz}
            local next_key = pos_key(next_pos)
            
            if not self.visited[next_key] then
                self.pending_move = {type="forward", d=d}
                return d.cmd
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
    debug = false
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
    
    -- Initial Look / Open
    MudExplorer.open_all()
    mud.send("l")
end

function MudExplorer.open_all()
    mud.send("op n"); mud.send("op s")
    mud.send("op e"); mud.send("op w")
    mud.send("op u"); mud.send("op d")
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
        s.exploring = false
        if s.callback then s.callback(true, line) end
        return
    end

    -- 2. Check Exits (Trigger Move)
    if string.find(line, "%[Exits:") or string.find(line, "%[出口:") then
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
    if not s.exploring then return end
    
    -- If we just arrived and haven't opened doors, open them and look again?
    -- Logic from MobFinder:
    local inst = s.instance
    if inst.pending_move and inst.pending_move.type == "forward" and not s.doors_opened then
        s.doors_opened = true
        MudExplorer.open_all()
        mud.send("l") -- Re-look
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
            s.instance.visited[pos_key(s.instance.pos)] = true -- Mark start as visited?
            -- Or just reset visited?
            -- MobFinder logic: reset visited, keep room_count?
            s.instance.visited = {}
            s.instance.visited[pos_key(s.instance.pos)] = true
            
            MudExplorer.process_next_step() -- Recursively call to find next move from start
        end
    end
end

return MudExplorer
