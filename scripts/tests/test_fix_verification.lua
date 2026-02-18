-- Test Fix Verification
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")
local MudNav = require("scripts/modules/MudNav")
-- ItemFarm is a global singleton usually, check if we can require it safely
-- or if we need to mock it entirely.
-- scripts/itemfarm.lua defines _G.ItemFarm. Let's try to load it.
-- But it might depend on other things. Let's see.
local status, ItemFarm = pcall(require, "scripts/itemfarm")
-- If require fails because it expects global 'mud' to exist during load time
-- we might need to set _G.mud first.

_G.describe("Fix Verification", function()
    local mock
    
    local function setup()
        mock = MockMud.new()
        _G.mud = mock
        -- Reset MudNav
        MudNav.reset()
        -- Reset ItemFarm state if available
        if _G.ItemFarm then
            _G.ItemFarm.state = {
                running = true,
                stage = "idle",
                run_id = "test_run",
                current_hp = 100, max_hp = 100,
                current_mp = 100, max_mp = 100
            }
            -- Mock config
            _G.ItemFarm.config = {
                hp_threshold = 50,
                mp_threshold = 50,
                path_to_storage = "n"
            }
            -- Mock job
            _G.ItemFarm.job = function() 
                return { 
                    name = "TestJob", 
                    hp_threshold = 0, 
                    mp_threshold = 0 
                }
            end
        end
        MudNav.config.refresh_cmd = "c ref"
        
        -- Mock os.clock to use mock time for flood control testing
        _G.os.clock = function() return mock.current_time end
    end
    
    _G.describe("MudNav Fixes", function()
        _G.it("should not flood c ref on multiple exhausted messages", function()
            setup()
            
            -- Simulate exhaustion
            MudNav.state.walking = true -- Required for on_server_message to process
            MudNav.on_server_message("You are exhausted")
            
            -- Should send 'c ref'
            local sent_count = #mock.sent
            assert_equal(1, sent_count, "Should send 1 command (c ref)")
            assert_equal("c ref", mock.sent[1], "Command should be 'c ref'")
            assert_equal(true, MudNav.state.paused, "Should be paused")
            
            -- Simulate second exhaustion message (flood)
            MudNav.on_server_message("You are exhausted")
            
            -- Should NOT send another 'c ref'
            assert_equal(1, #mock.sent, "Should still have only 1 command")
            
            -- Simulate third exhaustion message
            MudNav.on_server_message("移動力不足")
             assert_equal(1, #mock.sent, "Should still have only 1 command")
        end)
        
        _G.it("should not flood movement on multiple recovering messages", function()
            setup()
            
            -- Start a walk
            MudNav.walk("n;s", function() end)
            -- Initial 'n' sent
            mock.sent = {} -- Clear for easier checking
            MudNav.state.paused = true -- Simulate paused state
            
            -- Simulate recovery
            MudNav.on_server_message("You feel your stamina recovering")
            
            -- Should unpause and schedule next move
            assert_equal(false, MudNav.state.paused, "Should be unpaused")
            
            -- Tick timer (0.5s delay)
            mock:tick(0.6)
            
            -- Should have sent 'n' (first step again or next step, depending on logic)
            -- Actually MudNav.walk sets index=1. send_next sends index 1 'n'.
            -- If we paused, usually it means we failed to move or just paused.
            -- If we call send_next, it sends current index.
            assert_equal(1, #mock.sent, "Should send 1 movement command")
            assert_equal("n", mock.sent[1], "Previous command should be retried or next one sent")
            
            -- Reset sent
            mock.sent = {}

            -- Simulate second recovering message (flood)
            MudNav.on_server_message("You feel your stamina recovering")
            
            -- Tick timer again
            mock:tick(0.6)
            
            -- Should NOT send another command because we are already not paused?
            -- Wait, the logic is: if s.paused then s.paused=false; timer(send_next) end
            -- Since s.paused is now false, the second message should NOT trigger timer.
            
            assert_equal(0, #mock.sent, "Should NOT send duplicated movement command")
        end)
    end)
    
    if _G.ItemFarm then
        _G.describe("ItemFarm Fixes", function()
            _G.it("should switch to buffing stage when waiting for buffs", function()
                setup()
                
                -- Mock verify functions
                _G.ItemFarm.check_run = function() return true end
                _G.ItemFarm.safe_timer = function(t, cb) mock:timer(t, cb) end
                _G.ItemFarm.walk_path = function() end
                
                -- Mock check_and_apply_buffs to return "waiting"
                _G.ItemFarm.check_and_apply_buffs = function() return "waiting" end
                
                -- Set stage to checking
                _G.ItemFarm.state.stage = "checking_status_pre_fight"
                
                -- Execute
                _G.ItemFarm.evaluate_status_and_fight("test_run")
                
                -- Assert stage changed to "buffing"
                assert_equal("buffing", _G.ItemFarm.state.stage, "Stage should be 'buffing' to prevent Ok loop")
            end)
            
             _G.it("should switch to buffing stage when applying buffs", function()
                setup()
                 -- Mock verify functions
                 _G.ItemFarm.check_run = function() return true end
                 _G.ItemFarm.safe_timer = function(t, cb) mock:timer(t, cb) end
                
                -- Mock check_and_apply_buffs to return false (applying)
                _G.ItemFarm.check_and_apply_buffs = function() return false end
                
                _G.ItemFarm.state.stage = "checking_status_pre_fight"
                _G.ItemFarm.evaluate_status_and_fight("test_run")
                
                assert_equal("buffing", _G.ItemFarm.state.stage, "Stage should be 'buffing'")
            end)
        end)
    else
        print("⚠️ ItemFarm module not loaded, skipping ItemFarm tests")
    end

end)
