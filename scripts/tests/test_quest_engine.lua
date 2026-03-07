-- Test QuestEngine
package.path = "scripts/?.lua;scripts/?/init.lua;" .. package.path
if not _G.describe then dofile("scripts/tests/test_runner.lua") end
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

-- Load remaining modules for hunt tests
dofile("scripts/modules/MudExplorer.lua")
package.loaded["scripts.modules.MudExplorer"] = _G.MudExplorer
package.loaded["modules.MudExplorer"] = _G.MudExplorer
package.loaded["MudExplorer"] = _G.MudExplorer

dofile("scripts/modules/MudCombat.lua")
package.loaded["scripts.modules.MudCombat"] = _G.MudCombat
package.loaded["modules.MudCombat"] = _G.MudCombat
package.loaded["MudCombat"] = _G.MudCombat

dofile("scripts/modules/MudLoot.lua")
package.loaded["scripts.modules.MudLoot"] = _G.MudLoot
package.loaded["modules.MudLoot"] = _G.MudLoot
package.loaded["MudLoot"] = _G.MudLoot

-- Reload QuestEngine to pick up hunt handler with all deps available
dofile("scripts/modules/QuestEngine.lua")

describe("QuestEngine Hunt", function()
    it("should configure MudExplorer and start exploration", function()
        mock.sent = {}
        mock.logs = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("hunt_test", {
            steps = {
                {type="hunt", name="find_mob",
                 target="Goblin", attack_cmd="kill goblin",
                 loot={items={"gold"}, sac=true}},
            }
        })
        QuestEngine.run("hunt_test")

        assert_equal("Goblin", MudExplorer.config.target, "Explorer target should be set")
        assert_equal(true, MudExplorer.state.exploring, "Explorer should be active")
    end)
end)

describe("QuestEngine Simple Handlers", function()
    it("give handler should send 'gi item npc'", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("give_test", {
            steps = {{type="give", name="give_stone", item="stone", npc="king"}},
        })
        QuestEngine.run("give_test")
        local found = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "gi stone king" then found = true end
        end
        assert_equal(true, found, "Should send 'gi stone king'")
    end)

    it("say handler should send command", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("say_test", {
            steps = {{type="say", name="greet", text="hello world"}},
        })
        QuestEngine.run("say_test")
        local found = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "say hello world" then found = true end
        end
        assert_equal(true, found, "Should send 'say hello world'")
    end)

    it("interact handler should send custom cmd", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("interact_test", {
            steps = {{type="interact", name="push", cmd="push stone"}},
        })
        QuestEngine.run("interact_test")
        local found = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "push stone" then found = true end
        end
        assert_equal(true, found, "Should send 'push stone'")
    end)
end)

describe("QuestEngine Recall", function()
    it("should send recall before first step", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("recall_test", {
            recall_cmd = "recall",
            steps = {{type="interact", name="test", cmd="test"}},
        })
        QuestEngine.run("recall_test")
        local has_wa = false
        local has_recall = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "wa" then has_wa = true end
            if cmd == "recall" then has_recall = true end
        end
        assert_equal(true, has_wa, "Should send 'wa'")
        assert_equal(true, has_recall, "Should send 'recall'")
    end)
end)

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
