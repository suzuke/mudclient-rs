-- Test MudExplorer
local MockMud = require("scripts/tests/mock_mud")
local MudExplorer = require("scripts/modules/MudExplorer")

_G.describe("MudExplorer", function()
    
    _G.it("should explore a simple Layout", function()
        local mock = MockMud.new()
        _G.mud = mock
        
        local explorer = MudExplorer.new()
        explorer:start()
        
        -- Room A (Start): Exits: North
        -- Explorer should decide to go North
        -- We need to feed it room info.
        
        local next_cmd = explorer:process_room("[Exits: north]")
        assert_equal("n", next_cmd, "Should move north into only exit")
        
        -- Room B: Exits: South (Back to A)
        -- Visited: A(0,0,0), B(0,1,0). 
        -- B has only South, which leads to A (visited). 
        -- Should backtrack.
        -- Assuming DFS stack has {cmd="n", rev="s"}
        
        -- We need to update position in explorer? 
        -- Implementation detail: process_room usually handles the logic "I arrived".
        
        local backtrack_cmd = explorer:process_room("[Exits: south]")
        assert_equal("s", backtrack_cmd, "Should backtrack south")
        
        -- Back at Room A
        -- Exits: North (Visited B).
        -- No other exits.
        -- Path stack is empty.
        -- Should be Done.
        
        local final_cmd = explorer:process_room("[Exits: north]")
        assert_equal(nil, final_cmd, "Should be done")
        assert_equal(true, explorer.is_done, "Explorer should be marked done")
    end)

end)
