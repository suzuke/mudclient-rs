-- Test MobFinder
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")
-- We will load the refactored MobFinder later, but for now we test the concept
-- or we can test the existing MobFinder if we can load it? 
-- The existing MobFinder depends on global mud quite a bit.
-- We will write tests assuming the NEW structure which injects dependencies or uses the global mocks.

-- Since MobFinder.lua is not yet refactored to use Modules properly (it has its own logic),
-- and we are about to rewrite it. 
-- The strategy is: Write tests for the *expected* behavior of the NEW MobFinder.

local MobFinder -- Will require after setup

_G.describe("MobFinder", function()
    local mock
    
    local function setup()
        mock = MockMud.new()
        _G.mud = mock
        
        -- Reset package loaded to force reload of MobFinder if needed?
        -- For now, just require it.
        -- But MobFinder relies on MudNav/MudExplorer which use MudUtils.
        MudUtils.get_new_run_id()
        
        -- We need to mock MudNav and MudExplorer behaviors?
        -- Or just use the real ones with MockMud?
        -- Using real ones is better for integration testing.
        
        package.loaded["scripts/modules/MudNav"] = nil
        package.loaded["scripts/modules/MudExplorer"] = nil
        package.loaded["scripts/mob_finder"] = nil
        
        require("scripts/modules/MudNav")
        require("scripts/modules/MudExplorer")
        -- We will be testing the Refactored MobFinder, so we expect it to exist.
        -- Current file is old style. 
    end
    
    -- Helper to force reload modules (simulating clean state)
    local function reload_modules()
        for k, _ in pairs(package.loaded) do
            if k:match("MudUtils") or k:match("MudNav") or k:match("MudExplorer") or k:match("mob_finder") then
                package.loaded[k] = nil
            end
        end
        
        require("scripts.modules.MudUtils")
        require("scripts.modules.MudNav")
        require("scripts.modules.MudExplorer")
    end

    _G.it("should travel to area and search", function()
        setup()
        reload_modules()
        
        -- Load formatted MobFinder (future state)
        local MobFinder = require("scripts.mob_finder")
        local MudNav = require("scripts.modules.MudNav")
        local MudExplorer = require("scripts.modules.MudExplorer")
        
        -- Mock Config
        MobFinder.config.entry_path = "n;e"
        MobFinder.config.target = "Otonashi"
        
        -- Start
        MobFinder.start("Otonashi")
        
        -- Check initial sequence: repo -> wa -> recall -> timer(1.5) -> enter_area
        assert_equal("repo", mock.sent[1])
        assert_equal("wa", mock.sent[2])
        assert_equal("recall", mock.sent[3])
        
        
        -- Advance timer for enter_area
        mock:tick(1.6)
        
        print("DEBUG: mock.sent count: " .. #mock.sent)
        for i, v in ipairs(mock.sent) do print("  ["..i.."] " .. tostring(v)) end
        
        -- Should start walking entry path "n;e"
        -- MudNav walking...
        -- Need to simulate room descriptions for MudNav to proceed? 
        -- Or just basic commands if MudNav is used.
        -- If MudNav is used, it sends "n".
        assert_equal("n", mock.sent[4])
        
        -- Simulate arrival at "n" (room description with Exits)
        MudNav.config.debug = true
        print("DEBUG: MudNav.state.paused = " .. tostring(MudNav.state.paused))
        MudNav.on_server_message("[Exits: North South]")
        
        print("DEBUG: mock.logs count: " .. #mock.logs)
        for i, l in ipairs(mock.logs) do print("  Log["..i.."] " .. tostring(l)) end
        
        print("DEBUG: Pre-tick 2 timers count: " .. #mock.timers)
        for i, t in ipairs(mock.timers) do
            print("  Timer["..i.."] trigger=" .. t.trigger_time .. " code="..t.code)
        end
        
        mock:tick(0.6) -- Walk delay
        
        assert_equal("e", mock.sent[5])
        
        -- Simulate arrival at "e" (destination)
        MudNav.on_server_message("[Exits: East West]")
        mock:tick(0.6)
        
        -- Walk complete. Should trigger enter_area_done -> start_explore
        -- enter_area_done sends enter_cmds (empty) then timer(0.5) to start_explore
        mock:tick(0.6)
        
        -- start_explore should trigger MudExplorer
        -- MudExplorer starts with "l" usually? Or current logic "op n" etc.
        -- Refactored MobFinder should use MudExplorer.explore()
        -- MudExplorer.explore() does "l" first.
        
        -- Check if "l" was sent (or whatever MudExplorer does)
        local last_cmd = mock.sent[#mock.sent]
        assert_equal("l", last_cmd) 
        
        -- Simulate finding target
        MudExplorer.on_server_message("Otonashi is here.")
        
        -- Should stop? 
        -- MudExplorer Logic needed here.
    end)
end)
