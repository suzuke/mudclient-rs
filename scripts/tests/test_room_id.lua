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
    
    -- Test Strict vs Lax
    local id_strict = mud.get_room_id(name, desc, exits, true)
    local id_lax = mud.get_room_id(name, desc, exits, false)
    
    if id_strict == id_lax then
        -- They might be accidentally same if hash algo is broken, but generally should differ
        -- Actually, SHA256(A+B+C) vs SHA256(A+B) are definitely different
        mud.echo("Error: Strict and Lax IDs should be different")
        return
    end

    local exits_changed = {"north", "south", "up"}
    local id_lax_changed = mud.get_room_id(name, desc, exits_changed, false)
    
    if id_lax ~= id_lax_changed then
        mud.echo("Error: Lax ID should ignore exits")
        return
    end

    mud.echo("mud.get_room_id() tests passed!")
end

function test_current_room_api()
    mud.echo("Testing mud.get_current_room()...")
    local room = mud.get_current_room()
    if room then
        mud.echo("Current Room Info:")
        mud.echo("  Name: " .. room.name)
        mud.echo("  Desc Type: " .. type(room.description))
        mud.echo("  Exits Count: " .. #room.exits)
    else
        mud.echo("Current Room is nil (expected if no room detected yet)")
    end
end

function test_current_room()
    mud.echo("Testing mud.get_current_room_id()...")
    local current = mud.get_current_room_id()
    mud.echo("Current Room ID: " .. tostring(current))
    
    if current == nil then
        mud.echo("Note: Current Room ID is nil (expected if no room detected yet)")
    end

    local lax = mud.get_current_room_id(false)
    if lax then
        mud.echo("Current Room Lax ID: " .. tostring(lax))
        if current == lax and current ~= nil then
             -- usually diff unless no exits
             mud.echo("Note: Strict and Lax ID match (no exits?)")
        end
    end
end

test_room_id()
test_current_room()
test_current_room_api()
