-- Test MudNav
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")
local MudNav = require("scripts/modules/MudNav")

_G.describe("MudNav", function()
    local mock
    
    local function setup()
        mock = MockMud.new()
        _G.mud = mock
        -- MudUtils needs to be reset or re-initialized if it holds global state
        -- For now, we assume MudNav uses MudUtils.run_id
        MudUtils.get_new_run_id()
    end
    
    _G.it("should walk a simple path", function()
        setup()
        
        -- Override MudNav's internal timer delay to speed up test? 
        -- Or just tick the mock enough.
        
        local finished = false
        MudNav.walk("n;e", function() finished = true end)
        
        -- Initial state: first command should be sent immediately or after small delay?
        -- Usually walk starts immediately.
        
        -- If implementation uses timer 0.05 for first step:
        mock:tick(0.1)
        assert_equal("n", mock.sent[1], "First command should be 'n'")
        
        -- Simulate arrival (prompt trigger)
        -- MudNav should be listening to server messages. 
        -- We need to simulate the hook. 
        -- For this test, we assume MudNav exposes a function to call when prompt arrives
        -- or we mock the hook mechanism.
        
        -- Let's assume MudNav.on_server_message handles "[Exits: ...]"
        MudNav.on_server_message("[Exits: north south]")
        
        -- Should trigger next step after delay (e.g. 0.5s)
        mock:tick(0.6)
        
        assert_equal("e", mock.sent[2], "Second command should be 'e'")
        
        -- Finish
        MudNav.on_server_message("[Exits: east west]")
        mock:tick(0.6)
        
        -- Callback should be called
        assert_equal(true, finished, "Callback should be called")
    end)
    
    _G.it("should pause on stamina low", function()
        setup()
        MudNav.walk("n;n;n")
        mock:tick(0.1)
        assert_equal("n", mock.sent[1])
        
        -- Simulate stamina low input
        MudNav.on_server_message("You are exhausted")
        
        -- Should send 'c ref' (or whatever config says)
        -- And should NOT send next 'n' even if we tick
        
        -- Check if 'c ref' sent
        local sent_count_before = #mock.sent
        -- Depending on implementation, it might send immediately
        
        -- Let's assume it sends 'c ref'
        local last_cmd = mock.sent[#mock.sent]
        -- Note: Implementation detail, might vary. 
        -- If we haven't implemented MudNav yet, this test defines expectations.
        
        -- Logic: 
        -- 1. Pause walking
        -- 2. Send 'c ref'
        
        -- Simulate recovery
        MudNav.on_server_message("You feel your stamina recovering")
        
        -- Should resume
        mock:tick(1.0)
        -- Should see next 'n'
    end)

end)
