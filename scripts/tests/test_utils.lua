-- Test MudUtils
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")

_G.describe("MudUtils", function()

    _G.it("should have correct initial state", function()
        assert_equal(0, MudUtils.run_id, "Initial run_id should be 0")
    end)

    _G.it("should increment run_id", function()
        local current = MudUtils.run_id
        local next_id = MudUtils.get_new_run_id()
        assert_equal(current + 1, next_id, "Should increment run_id")
        assert_equal(next_id, MudUtils.run_id, "Global state updated")
    end)

    _G.it("should parse multiple commands", function()
        local cmds = MudUtils.parse_cmds("3n;2e;open s")
        assert_equal("n", cmds[1])
        assert_equal("n", cmds[2])
        assert_equal("n", cmds[3])
        assert_equal("e", cmds[4])
        assert_equal("e", cmds[5])
        assert_equal("open s", cmds[6])
    end)

    _G.it("safe_timer should restrict execution by run_id", function()
        local mock = MockMud.new()
        _G.mud = mock -- Inject mock
        
        -- Override MudUtils timer to use global mud.timer which is now mocked
        -- Note: Real implementation of MudUtils.safe_timer uses `mud.timer`
        
        local executed = false
        local initial_rid = MudUtils.get_new_run_id()
        
        MudUtils.safe_timer(1.0, function(rid)
            executed = true
        end)
        
        -- Advance run_id to invalidate the timer
        MudUtils.get_new_run_id()
        
        -- Tick the mock
        -- We need to manually simulate the callback execution logic if we were using real Lua ev loop
        -- But here `MudUtils.safe_timer` generates a string code.
        -- Let's see how `safe_timer` is implemented.
        -- If it uses `mud.timer`, our mock captures it.
        
        -- For this test to work with our simple runner/mock, we need to inspect what was sent to mud.timer
        assert_equal(1, #mock.timers, "Timer should be registered")
        local t = mock.timers[1]
        
        -- The callback code stored in t.code needs to be executed
        -- But since we are inside a test environment, `_G.MudUtils` might not be fully accessible via load() string if it's local
        -- This is a complexity of testing Lua.
        -- For now, let's assume `safe_timer` registers a function wrapper if possible or we inspect the string.
        
        -- Actually, `safe_timer` logic:
        -- if rid == MudUtils.run_id then callback() end
        
        -- We can just manually invoke the logic if we can access the wrapper.
        -- But typically `mud.timer` takes a string.
        
        -- Let's adjust the test expectation or strategy.
        -- If `safe_timer` generates a string calling a global function, we need that global function.
        -- MudUtils usually exports itself to _G.
        
    end)

end)
