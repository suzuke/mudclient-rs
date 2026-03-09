-- Test ItemFarm
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")

_G.describe("ItemFarm", function()
    local mock
    
    local function reload_modules()
        for k, _ in pairs(package.loaded) do
            if k:match("itemfarm") or k:match("MudUtils") or k:match("MudNav") or k:match("MudCombat") then
                package.loaded[k] = nil
            end
        end
        require("scripts.modules.MudUtils")
        require("scripts.modules.MudNav")
        require("scripts.modules.MudCombat")
    end

    local function setup()
        mock = MockMud.new()
        _G.mud = mock
        reload_modules()
    end

    _G.it("should run a full summon job cycle", function()
        setup()
        local ItemFarm = require("scripts.itemfarm")
        
        -- Override config to be faster and simpler
        ItemFarm.config.poll_interval = 1
        ItemFarm.config.show_echo = false
        
        -- Override jobs to have just ONE simple job
        ItemFarm.jobs = {
            {
                name = "Test Job",
                mode = "summon",
                search_type = "quest",
                search_cmd = "q test",
                target_mob = "TestMob",
                summon_cmd = "c sum test",
                attack_cmd = "kill test",
                path_to_mob = "n;e",
                path_to_storage = "s;w",
                loot_items = {"gold"},
                remove_nodrop = {},
                sac_corpse = true,
            }
        }
        
        -- Start
        ItemFarm.start()
        
        -- 1. Search Phase
        -- expect "q test"
        assert_equal("q test", mock.sent[#mock.sent])
        
        -- Simulate Found
        ItemFarm.on_server_message("他正在這個世界中", "他正在這個世界中") -- Trigger found
        mock:tick(1.1) -- Wait for timer (1.0s) to go_and_fight
        
        -- 2. Travel Phase
        -- ItemFarm (Old) uses prompt-driven walk.
        -- We verify it sends the path commands "n", "e"
        -- It sends "wa" first usually?
        -- Code says: mud.send("wa") then walk_path("n;e")
        
        -- Depending on implementation (run_command vs run_command_async in walk_path),
        -- ItemFarm.walk_path sends first command immediately?
        -- Let's check mock.sent
        -- It should have "wa"
        -- Then "n"
        
        -- Note: OLD ItemFarm walk_path sends one command then waits for hook.
        -- But hook relies on what?
        -- OLD ItemFarm walk_path relies on 'prompt' detection?
        -- Or just walks?
        -- The code I read: wait for [出口:] or prompt?
        -- Actually OLD walk_path:
        -- function walk_send: mud.send(cmd) -> wait for hook -> walk_advance
        
        -- If I test the OLD code, I need to simulate prompts/exits for every step.
        -- If I am refactoring to MudNav, I'd rather test the NEW behavior expectation.
        
        -- BUT this test is running against the CURRENT (Old) code.
        -- So I must simulate what the old code expects to verify the baseline?
        -- Or I can just write the test expecting the NEW behavior and use it to drive the refactor.
        -- Given I already read the code, I know the goal is MudNav.
        
        -- Let's write the test expecting MUDNAV behavior (trigger based).
        -- This means the test will FAIL initially, which is fine (TDD). 
        -- Wait, if correct ItemFarm (old) uses prompts, and I simulate triggers, it might hang.
        
        -- Actually, verifying the *logic flow* (Job -> Summon -> Fight) is more important than the movement mechanics details for now.
        -- I can just Mock the movement function to jump to destination if I want to isolate logic.
        
        -- Mocking ItemFarm.walk_path?
        -- ItemFarm.walk_path = function(path, cb) 
        --    -- invoke callback immediately 
        --    local func = _G.ItemFarm[cb:gsub("_G.ItemFarm.", "")] or loadstring("return " .. cb)()
        --    func(ItemFarm.state.run_id)
        -- end
        
        -- This allows testing the state machine without fighting the legacy movement code.
        ItemFarm.walk_path = function(path, cb)
             -- Simulate delay
             mock:tick(0.1)
             local func_name = cb:match("ItemFarm%.(.+)")
             local func = ItemFarm[func_name]
             if func then func(ItemFarm.state.run_id) end
        end
        
        -- Re-Start to apply mock
        ItemFarm.stop()
        ItemFarm.start() -- Search
        ItemFarm.on_server_message("他正在這個世界中", "他正在這個世界中") -- Found
        mock:tick(1.1) -- Timer
        
        -- Should have called go_and_fight -> walk_path (mocked) -> callback (check_status_before_summon)
        
        -- check_status_before_summon sends "rep", "score aff", "save"
        -- Verify these commands
        local sent = mock.sent
        assert_equal("save", sent[#sent])
        
        -- Simulate "Ok." to trigger evaluation
        -- evaluate_status_before_summon checks HP/MP
        -- We need to mock HP/MP state or feed "rep" output
        ItemFarm.state.current_hp = 100
        ItemFarm.state.max_hp = 100
        ItemFarm.state.current_mp = 100
        ItemFarm.state.max_mp = 100
        
        ItemFarm.on_server_message("Ok.", "Ok.")
        
        -- Should call summon_and_attack -> "c sum test"
        mock:tick(0.1)
        assert_equal("c sum test", mock.sent[#mock.sent])
        
        -- 3. Summon Success
        ItemFarm.on_server_message("TestMob 突然出現在你的眼前", "TestMob 突然出現在你的眼前")
        mock:tick(1.1) -- Wait MudCombat verify delay (1.0s)
        
        -- Should call start_fighting -> "kill test"
        assert_equal("kill test", mock.sent[#mock.sent])
        
        -- 4. Kill
        ItemFarm.on_server_message("TestMob 魂歸西天了", "TestMob 魂歸西天了")
        mock:tick(0.6)
        
        -- Should loot ("get gold corpse")
        assert_equal("get gold corpse", mock.sent[#mock.sent-1]) -- sac corpse is last
        assert_equal("sac corpse", mock.sent[#mock.sent])
        
        -- Should go to storage (mocked walk) -> remove_and_drop -> drop items
        mock:tick(1.1)
        
        -- Should drop items ("dro gold")
        assert_equal("dro gold", mock.sent[#mock.sent])
        
        -- Should rest
        mock:tick(2.1)
        assert_equal("sleep", mock.sent[#mock.sent])
        
    end)
end)
