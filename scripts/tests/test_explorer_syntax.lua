-- Mock mud object
_G.mud = {
    echo = function(msg) print("MUD ECHO: " .. msg) end,
    send = function(cmd) print("MUD SEND: " .. cmd) end,
    timer = function() end,
    get_current_room_id = function() return "mock_id_explorer_1" end
}

-- Setup package path to include modules
package.path = package.path .. ";./?.lua;./modules/?.lua;../?.lua"

-- Clean package.loaded
package.loaded["modules.MudUtils"] = nil
package.loaded["modules.MudExplorer"] = nil

local status, MudExplorer = pcall(require, "modules.MudExplorer")
if not status then
    print("Error loading MudExplorer: " .. tostring(MudExplorer))
    os.exit(1)
end

print("MudExplorer loaded successfully")

-- Test new instance
local explorer = MudExplorer.new()
if explorer.visited_ids then
    print("explorer.visited_ids exists")
else
    print("Error: explorer.visited_ids missing")
    os.exit(1)
end

-- Test state
MudExplorer.state.exploring = true
MudExplorer.state.instance = explorer

-- Simulation
explorer:start()

-- Test process_room with mock ID
-- function parse_exits needs to work.
-- Mocking mud.get_current_room_id is done in _G.mud
-- But MudExplorer calls `mud.get_current_room_id` directly?
-- It calls `mud.get_current_room_id()`. 
-- In our mock, `mud` is global, so it should work if MudExplorer accesses global `mud`.
-- However, `MudExplorer.lua` does NOT check `_G.mud` explicitly in `process_room`, it assumes `mud` exists.
-- Let's check `MudExplorer.lua` content...
-- It calls `mud.get_current_room_id()`.

explorer:process_room("[Exits: north]")
print("explorer:process_room executed")

if explorer.visited_ids["mock_id_explorer_1"] then
    print("visited_ids updated correctly")
else
    print("Error: visited_ids not updated")
    os.exit(1)
end
