-- Test QuestEngine
package.path = "scripts/?.lua;scripts/?/init.lua;" .. package.path
dofile("scripts/tests/test_runner.lua")
local MockMud = dofile("scripts/tests/mock_mud.lua")
local mock = MockMud.new()
_G.mud = mock

-- Add mocks not in MockMud
_G.mud.emit = function(event, data) end
_G.mud.start_log = function(f) end
_G.mud.stop_log = function() end
_G.mud.get_current_room_id = function() return nil end

dofile("scripts/modules/MudUtils.lua")
MudUtils.run_id = 0
MudUtils.callbacks = {}
MudUtils.callback_id = 0
MudUtils.active_quests = {}

dofile("scripts/modules/QuestEngine.lua")

describe("QuestEngine", function()

    it("should define a quest and store it", function()
        QuestEngine.quests = {}
        local def = {
            steps = {
                { type = "navigate", target = "room1" },
                { type = "command", cmd = "look" },
            }
        }
        QuestEngine.define("test_quest", def)
        assert_equal(def, QuestEngine.quests["test_quest"], "Quest should be stored")
        assert_equal(2, #QuestEngine.quests["test_quest"].steps, "Quest should have 2 steps")
    end)

    it("should start a quest and set state", function()
        -- Reset
        QuestEngine.quests = {}
        QuestEngine.state = { running = false, quest_name = nil, step_index = 0, run_id = 0, phase = nil }
        MudUtils.run_id = 0

        -- Register a dummy handler so execute_step doesn't fail
        QuestEngine.handlers["navigate"] = function(step) end

        local def = {
            steps = {
                { type = "navigate", target = "room1" },
            }
        }
        QuestEngine.define("test_quest", def)
        QuestEngine.run("test_quest")

        assert_equal(true, QuestEngine.state.running, "Should be running")
        assert_equal("test_quest", QuestEngine.state.quest_name, "Quest name should match")
        assert_equal(1, QuestEngine.state.step_index, "Step index should be 1")
        assert_equal("running", QuestEngine.state.phase, "Phase should be running")
    end)

    it("should stop a quest and reset state", function()
        -- Reset and start
        QuestEngine.quests = {}
        QuestEngine.state = { running = false, quest_name = nil, step_index = 0, run_id = 0, phase = nil }
        MudUtils.run_id = 0

        QuestEngine.handlers["command"] = function(step) end

        local def = {
            steps = {
                { type = "command", cmd = "look" },
            }
        }
        QuestEngine.define("stop_test", def)
        QuestEngine.run("stop_test")
        assert_equal(true, QuestEngine.state.running, "Should be running before stop")

        QuestEngine.stop(true)
        assert_equal(false, QuestEngine.state.running, "Should not be running after stop")
        assert_equal(0, QuestEngine.state.step_index, "Step index should be 0 after stop")
    end)

end)

-- Load MudNav for navigate tests
dofile("scripts/modules/MudNav.lua")
package.loaded["scripts.modules.MudNav"] = _G.MudNav
package.loaded["modules.MudNav"] = _G.MudNav
package.loaded["MudNav"] = _G.MudNav

-- Reload QuestEngine to re-register the navigate handler (earlier tests overwrite it)
dofile("scripts/modules/QuestEngine.lua")

describe("QuestEngine Navigate", function()
    it("should call MudNav.walk with the path", function()
        -- Reset state
        mock.sent = {}
        mock.logs = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("nav_test", {
            steps = {
                {type="navigate", name="go_south", path={"s", "s", "e"}},
            }
        })
        QuestEngine.run("nav_test")

        -- MudNav.walk should send first command
        local found_s = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "s" then found_s = true end
        end
        assert_equal(true, found_s, "First nav command should be 's'")
    end)
end)
