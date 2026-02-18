-- Mock mud object
_G.mud = {
    echo = function(msg) print("MUD ECHO: " .. msg) end,
    send = function(cmd) print("MUD SEND: " .. cmd) end,
    timer = function() end,
    get_current_room_id = function() return "mock_id_123" end
}

-- Setup package path to include modules
-- We assume running from scripts/ directory
package.path = package.path .. ";./?.lua;./modules/?.lua;../?.lua"

-- Clean package.loaded
package.loaded["modules.MudUtils"] = nil
package.loaded["modules.MudNav"] = nil

-- Try loading MudNav
local status, MudNav = pcall(require, "modules.MudNav")
if not status then
    print("Error loading MudNav: " .. tostring(MudNav))
    os.exit(1)
end

print("MudNav loaded successfully")

-- Test new methods exist
if type(MudNav.record_start) == "function" then
    print("MudNav.record_start exists")
else
    print("Error: MudNav.record_start missing")
    os.exit(1)
end

if type(MudNav.walk) == "function" then
    print("MudNav.walk exists")
else
    print("Error: MudNav.walk missing")
    os.exit(1)
end

-- Test Walk with Table
local test_path = {
    {cmd="n", id="123"},
    {cmd="s", id="456"}
}

MudNav.walk(test_path, function() end)
print("MudNav.walk called with table path")
