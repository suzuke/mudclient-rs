-- Test MudCombat
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")
local MudCombat = require("scripts/modules/MudCombat")

_G.describe("MudCombat", function()
    local mock
    
    local function setup()
        mock = MockMud.new()
        _G.mud = mock
        MudUtils.get_new_run_id()
    end
    
    _G.it("should summon successfully", function()
        setup()
        local done = false
        
        MudCombat.safe_summon("Papa", "c sum papa", {}, function() done = true end, nil)
        
        -- Check command sent
        mock:tick(0.1)
        assert_equal("c sum papa", mock.sent[1])
        
        -- Simulate success msg
        MudCombat.on_server_message("Papa 突然出現在你的眼前")
        
        -- Wait verify delay (default 1.0)
        mock:tick(1.1)
        
        assert_equal(true, done, "Success callback should be called")
    end)
    
    _G.it("should retry on failure", function()
        setup()
        local done = false
        local fail = false
        
        MudCombat.safe_summon("Papa", "c sum papa", {retry_delay=0.1}, function() done = true end, function() fail = true end)
        
        mock:tick(0.1)
        assert_equal("c sum papa", mock.sent[1])
        
        -- Simulate fail
        MudCombat.on_server_message("你失敗了")
        
        -- Wait retry delay
        mock:tick(0.2)
        assert_equal("c sum papa", mock.sent[2], "Should retry summon")
        
        -- Simulate success
        MudCombat.on_server_message("Papa 突然出現在你的眼前")
        mock:tick(1.1) -- verify delay
        
        assert_equal(true, done, "Should succeed eventually")
        assert_equal(false, fail, "Should not fail")
    end)
    
    _G.it("should retry on fled", function()
        setup()
        local done = false
        
        MudCombat.safe_summon("Papa", "c sum papa", {retry_delay=0.1, verify_delay=0.5}, function() done = true end, nil)
        
        mock:tick(0.1)
        
        -- Simulate success then fled
        MudCombat.on_server_message("Papa 突然出現在你的眼前")
        MudCombat.on_server_message("Papa 往北邊離開了")
        
        -- Verify delay (0.5)
        mock:tick(0.6)
        
        -- At this point verify callback runs, checks fled, and schedules retry (0.1s delay)
        -- Current time is 0.6. Retry scheduled for 0.7.
        
        -- Tick again to trigger retry
        mock:tick(0.2)
        
        -- Debug output
        -- for i, cmd in ipairs(mock.sent) do print("Sent["..i.."]: " .. cmd) end
        
        assert_equal("c sum papa", mock.sent[2], "Should retry after fleeing")
        assert_equal("c sum papa", mock.sent[2], "Should retry after fleeing")
        
        -- Simulate final success
        MudCombat.on_server_message("Papa 突然出現在你的眼前")
        mock:tick(0.6)
        assert_equal(true, done)
    end)

end)
