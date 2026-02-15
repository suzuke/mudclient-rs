-- Test Room ID API

function test_room_id()
    mud.echo("Testing mud.get_room_id()...")
    
    local name = "Test Room"
    local desc = "This is a test room."
    local exits = {"north", "south"}
    
    local id1 = mud.get_room_id(name, desc, exits)
    mud.echo("ID1: " .. tostring(id1))
    
    if type(id1) ~= "string" then
        mud.echo("Error: ID should be string")
        return
    end
    
    if #id1 ~= 64 then
        mud.echo("Error: ID length should be 64 (SHA256 hex)")
        return
    end
    
    -- Test consistency (sorted exits)
    local exits2 = {"south", "north"}
    local id2 = mud.get_room_id(name, desc, exits2)
    
    if id1 ~= id2 then
        mud.echo("Error: Exits order should not matter")
        return
    end
    
    -- Test uniqueness
    local id3 = mud.get_room_id("Other Room", desc, exits)
    if id1 == id3 then
        mud.echo("Error: Different rooms should have different IDs")
        return
    end
    
    mud.echo("mud.get_room_id() tests passed!")
end

function test_current_room()
    mud.echo("Testing mud.get_current_room_id()...")
    local current = mud.get_current_room_id()
    mud.echo("Current Room ID: " .. tostring(current))
    
    if current == nil then
        mud.echo("Note: Current Room ID is nil (expected if no room detected yet)")
    end
end

test_room_id()
test_current_room()
