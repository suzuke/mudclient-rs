# QuestEngine Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a pure Lua declarative QuestEngine module that reduces 600+ line quest scripts to ~30 line data definitions.

**Architecture:** QuestEngine is a single Lua module (`scripts/modules/QuestEngine.lua`) that orchestrates existing modules (MudNav, MudExplorer, MudCombat, MudLoot, MudUtils) via callback chaining. Each step type (navigate, hunt, give, say, interact) has a dedicated handler function. Steps execute sequentially; each handler calls `advance()` on completion.

**Tech Stack:** Lua 5.4, existing MudUtils mock + test_runner.lua for tests

---

### Task 1: QuestEngine Core — define, state, advance

**Files:**
- Create: `scripts/modules/QuestEngine.lua`
- Create: `scripts/tests/test_quest_engine.lua`

**Step 1: Write the failing test**

Add `scripts/tests/test_quest_engine.lua`:

```lua
-- test_quest_engine.lua
package.path = "scripts/?.lua;scripts/?/init.lua;" .. package.path

-- Load test infrastructure
dofile("scripts/tests/test_runner.lua")
local MockMud = dofile("scripts/tests/mock_mud.lua")

-- Setup mock
local mock = MockMud.new()
_G.mud = mock

-- Load modules
dofile("scripts/modules/MudUtils.lua")

-- Reset MudUtils state for test isolation
MudUtils.run_id = 0
MudUtils.callbacks = {}
MudUtils.callback_id = 0
MudUtils.active_quests = {}

dofile("scripts/modules/QuestEngine.lua")

describe("QuestEngine Core", function()
    it("should define a quest and store it", function()
        QuestEngine.define("test_quest", {
            steps = {
                {type="say", name="step1", text="hello", expect="world"},
                {type="say", name="step2", text="bye", expect="see ya"},
            }
        })
        assert_equal(2, #QuestEngine.quests["test_quest"].steps)
    end)

    it("should start a quest and set state", function()
        -- Reset mock
        mock.sent = {}
        mock.logs = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("simple", {
            steps = {
                {type="say", name="greet", text="hello", expect="world"},
            }
        })
        QuestEngine.run("simple")
        local s = QuestEngine.state
        assert_equal(true, s.running)
        assert_equal(1, s.step_index)
        assert_equal("simple", s.quest_name)
    end)

    it("should stop a quest and reset state", function()
        QuestEngine.stop()
        assert_equal(false, QuestEngine.state.running)
    end)
end)
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/suzuke/Documents/Hack/mudclient-rs && lua scripts/tests/test_quest_engine.lua`
Expected: FAIL — `QuestEngine` not found

**Step 3: Write minimal implementation**

Create `scripts/modules/QuestEngine.lua`:

```lua
-- QuestEngine Module
-- Declarative quest orchestration over MudNav, MudExplorer, MudCombat, MudLoot

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("QuestEngine cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

local QuestEngine = {}

QuestEngine.quests = {}
QuestEngine.state = {
    running = false,
    quest_name = nil,
    step_index = 0,
    run_id = 0,
    phase = "idle",
}

function QuestEngine.define(name, def)
    QuestEngine.quests[name] = def
end

function QuestEngine.run(name)
    local def = QuestEngine.quests[name]
    if not def then
        mud.echo("[QuestEngine] Unknown quest: " .. name)
        return
    end

    local s = QuestEngine.state
    if s.running then
        mud.echo("[QuestEngine] A quest is already running: " .. s.quest_name)
        return
    end

    s.running = true
    s.quest_name = name
    s.step_index = 1
    s.run_id = MudUtils.get_new_run_id()
    s.phase = "starting"

    MudUtils.register_quest("QuestEngine:" .. name, function() QuestEngine.stop() end)
    MudUtils.start_log("quest_" .. name)
    mud.echo("[QuestEngine] Started: " .. name)
    mud.emit("quest_started", {name = name})

    QuestEngine.execute_step()
end

function QuestEngine.stop(is_success)
    local s = QuestEngine.state
    if not s.running then return end

    s.running = false
    s.phase = "stopped"
    MudUtils.get_new_run_id()
    MudUtils.stop_log()
    mud.echo("[QuestEngine] Stopped: " .. (s.quest_name or "?") .. (is_success and " (Success)" or " (Aborted)"))
    mud.emit("quest_stopped", {name = s.quest_name, success = is_success or false})

    -- Cleanup active modules
    local ok1, _ = pcall(function()
        local MudExplorer = require_module("MudExplorer")
        MudExplorer.stop()
    end)
    local ok2, _ = pcall(function()
        local MudNav = require_module("MudNav")
        MudNav.reset()
    end)
end

function QuestEngine.advance()
    local s = QuestEngine.state
    if not s.running then return end

    s.step_index = s.step_index + 1
    local def = QuestEngine.quests[s.quest_name]

    if s.step_index > #def.steps then
        mud.echo("[QuestEngine] All steps complete!")
        QuestEngine.stop(true)
        return
    end

    MudUtils.safe_timer(0.5, function()
        QuestEngine.execute_step()
    end)
end

function QuestEngine.execute_step()
    local s = QuestEngine.state
    if not s.running then return end

    local def = QuestEngine.quests[s.quest_name]
    local step = def.steps[s.step_index]

    if not step then
        QuestEngine.stop(true)
        return
    end

    mud.echo("[QuestEngine] Step [" .. s.step_index .. "]: " .. (step.name or step.type))

    local handler = QuestEngine.handlers[step.type]
    if handler then
        s.phase = step.type
        handler(step, def)
    else
        mud.echo("[QuestEngine] Unknown step type: " .. step.type)
        QuestEngine.stop(false)
    end
end

-- Step handlers table (populated in subsequent tasks)
QuestEngine.handlers = {}

-- Server message hook (populated in subsequent tasks)
function QuestEngine.on_server_message(line, clean_line)
    if not QuestEngine.state.running then return end
    -- Will be extended per step type
end

MudUtils.register_hook("QuestEngine", function(line, clean_line, is_echo)
    if is_echo then return end
    QuestEngine.on_server_message(clean_line or line)
end)

_G.QuestEngine = QuestEngine
return QuestEngine
```

**Step 4: Run test to verify it passes**

Run: `cd /Users/suzuke/Documents/Hack/mudclient-rs && lua scripts/tests/test_quest_engine.lua`
Expected: All 3 tests PASS

**Step 5: Commit**

```bash
git add scripts/modules/QuestEngine.lua scripts/tests/test_quest_engine.lua
git commit -m "feat: add QuestEngine core — define, run, stop, advance"
```

---

### Task 2: Navigate handler

**Files:**
- Modify: `scripts/modules/QuestEngine.lua` (add navigate handler)
- Modify: `scripts/tests/test_quest_engine.lua` (add navigate tests)

**Step 1: Write the failing test**

Append to `scripts/tests/test_quest_engine.lua`:

```lua
-- Need MudNav loaded
dofile("scripts/modules/MudNav.lua")

describe("QuestEngine Navigate Handler", function()
    it("should send path commands via MudNav.walk", function()
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

        -- MudNav.walk should have been called, first command sent
        assert_equal("s", mock.sent[#mock.sent], "First nav command should be 's'")
    end)

    it("should advance after navigate completes", function()
        -- Simulate MudNav callback with success
        -- MudNav internally calls its callback(true) when walk completes
        -- We can directly test by calling the stored callback
        local s = QuestEngine.state
        assert_equal(true, s.running)
    end)
end)
```

**Step 2: Run test to verify it fails**

Run: `lua scripts/tests/test_quest_engine.lua`
Expected: FAIL — navigate handler not found

**Step 3: Write minimal implementation**

Add to `QuestEngine.lua`, before the `return` statement:

```lua
-- ===== Navigate Handler =====
QuestEngine.handlers["navigate"] = function(step, def)
    local MudNav = require_module("MudNav")
    local s = QuestEngine.state
    s.phase = "navigating"

    local on_fail = step.on_fail or "stop"

    MudNav.walk(step.path, function(success, reason)
        if not s.running or s.run_id ~= MudUtils.run_id then return end

        if success then
            -- Execute post-arrival commands if any
            if step.cmds then
                for _, cmd in ipairs(step.cmds) do
                    mud.send(cmd)
                end
            end

            if step.expect then
                s.phase = "waiting_response"
                s.current_expect = step.expect
                MudUtils.safe_timer(10.0, function()
                    if s.running and s.phase == "waiting_response" then
                        mud.echo("[QuestEngine] Timeout waiting for: " .. step.expect)
                        QuestEngine.stop(false)
                    end
                end)
            else
                QuestEngine.advance()
            end
        else
            mud.echo("[QuestEngine] Navigate failed: " .. tostring(reason))
            if on_fail == "retry" then
                MudUtils.safe_timer(2.0, function() QuestEngine.execute_step() end)
            else
                QuestEngine.stop(false)
            end
        end
    end)
end
```

Also update `on_server_message` to handle `waiting_response`:

```lua
function QuestEngine.on_server_message(line, clean_line)
    if not QuestEngine.state.running then return end
    local s = QuestEngine.state
    local text = clean_line or line

    -- Sleep detection
    if string.find(text, "你正在睡覺") or string.find(text, "睡得很熟") then
        MudUtils.safe_timer(1.5, function() mud.send("wa") end)
        return
    end

    -- Generic expect matching (used by navigate+cmds, say, interact)
    if s.phase == "waiting_response" and s.current_expect then
        if string.find(text, s.current_expect, 1, true) then
            mud.echo("[QuestEngine] Matched: " .. s.current_expect)
            s.current_expect = nil
            QuestEngine.advance()
        end
    end
end
```

**Step 4: Run test to verify it passes**

Run: `lua scripts/tests/test_quest_engine.lua`
Expected: PASS

**Step 5: Commit**

```bash
git add scripts/modules/QuestEngine.lua scripts/tests/test_quest_engine.lua
git commit -m "feat: add QuestEngine navigate handler with expect support"
```

---

### Task 3: Hunt handler

**Files:**
- Modify: `scripts/modules/QuestEngine.lua`
- Modify: `scripts/tests/test_quest_engine.lua`

**Step 1: Write the failing test**

```lua
dofile("scripts/modules/MudExplorer.lua")
dofile("scripts/modules/MudCombat.lua")
dofile("scripts/modules/MudLoot.lua")

describe("QuestEngine Hunt Handler", function()
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
```

**Step 2: Run test to verify it fails**

Run: `lua scripts/tests/test_quest_engine.lua`
Expected: FAIL

**Step 3: Write minimal implementation**

```lua
-- ===== Hunt Handler =====
-- Flow: explore -> find target -> fight -> loot -> backtrack -> advance
QuestEngine.handlers["hunt"] = function(step, def)
    local MudExplorer = require_module("MudExplorer")
    local MudCombat = require_module("MudCombat")
    local MudLoot = require_module("MudLoot")
    local MudNav = require_module("MudNav")
    local s = QuestEngine.state

    -- Configure explorer
    MudExplorer.config.target = step.target
    MudExplorer.config.max_laps = step.max_laps or 5
    MudExplorer.config.disable_open_doors = step.disable_open_doors or false
    MudExplorer.config.debug = step.debug or false

    s.phase = "finding"
    s.hunt_step = step
    s.hunt_kills = 0
    s.hunt_got_item = false
    s.non_target_combat = false
    s.recovering = false

    mud.send("wa")

    local function on_explore_done(found, target_line)
        if not s.running or s.run_id ~= MudUtils.run_id then return end

        if found then
            mud.echo("[QuestEngine] Target found! Fighting...")
            s.phase = "fighting"
            s.non_target_combat = false
            mud.send("wa")
            MudUtils.safe_timer(0.5, function()
                if not s.running then return end
                mud.send(step.attack_cmd)
                QuestEngine.combat_heartbeat()
            end)
        else
            mud.echo("[QuestEngine] Exploration complete, target not found.")
            if step.on_not_found == "retry" then
                MudUtils.safe_timer(2.0, function() QuestEngine.execute_step() end)
            else
                QuestEngine.stop(false)
            end
        end
    end

    s.hunt_explore_cb = on_explore_done
    MudExplorer.explore(on_explore_done)
end

-- Combat heartbeat for hunt
function QuestEngine.combat_heartbeat()
    local s = QuestEngine.state
    if not s.running or s.phase ~= "fighting" then return end

    if not s.recovering and s.hunt_step then
        if s.non_target_combat then
            mud.send("flee")
        else
            mud.send(s.hunt_step.attack_cmd)
        end
    end

    MudUtils.safe_timer(2.5, function()
        QuestEngine.combat_heartbeat()
    end)
end

-- Check combat clear for hunt
function QuestEngine.check_combat_clear()
    local s = QuestEngine.state
    if not s.running or s.phase ~= "clearing" then return end
    local MudCombat = require_module("MudCombat")

    s.clear_checks = (s.clear_checks or 0) + 1
    if not MudCombat.is_fighting() or s.clear_checks >= 10 then
        s.clear_checks = 0
        QuestEngine.handle_hunt_loot()
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.check_combat_clear() end)
    end
end

-- Loot after hunt kill
function QuestEngine.handle_hunt_loot()
    local s = QuestEngine.state
    if not s.running then return end
    local MudLoot = require_module("MudLoot")

    local loot_opts = s.hunt_step.loot or {}
    s.phase = "looting"

    MudLoot.process_loot({
        items = loot_opts.items or {"all"},
        loot_ground = loot_opts.loot_ground or true,
        sac = loot_opts.sac or false,
        fallback_blind = loot_opts.fallback_blind or true,
    }, function()
        QuestEngine.handle_hunt_after_loot()
    end)
end

-- After loot: backtrack or continue exploring
function QuestEngine.handle_hunt_after_loot()
    local s = QuestEngine.state
    if not s.running then return end
    local MudExplorer = require_module("MudExplorer")
    local MudNav = require_module("MudNav")

    if s.hunt_got_item or (s.hunt_step.kill_count and s.hunt_kills >= s.hunt_step.kill_count) then
        -- Done hunting, backtrack to start and advance
        local backtrack = MudExplorer.get_path_to_start()
        MudExplorer.stop()

        if backtrack and backtrack ~= "" then
            s.phase = "backtracking"
            MudNav.walk(backtrack, function()
                if not s.running then return end
                QuestEngine.advance()
            end)
        else
            QuestEngine.advance()
        end
    else
        -- Continue exploring
        s.phase = "finding"
        MudExplorer.resume(s.hunt_explore_cb)
    end
end
```

Also extend `on_server_message` for hunt combat detection:

```lua
-- Inside on_server_message, add hunt-specific combat monitoring:

    -- Hunt: combat detection
    if s.phase == "fighting" or s.phase == "finding" or s.phase == "clearing" then
        local MudCombat = require_module("MudCombat")
        if MudCombat.on_server_message(text) then
            if s.phase == "finding" then
                s.non_target_combat = not string.find(text, s.hunt_step.target, 1, true)
                s.phase = "fighting"
                QuestEngine.combat_heartbeat()
            end
        end
    end

    -- Hunt: kill detection
    if s.phase == "fighting" and string.find(text, "魂歸西天了") then
        if s.hunt_step and string.find(text, s.hunt_step.target, 1, true) then
            s.hunt_kills = s.hunt_kills + 1
            mud.echo("[QuestEngine] Kill #" .. s.hunt_kills)
            s.phase = "clearing"
            MudUtils.safe_timer(1.0, function() QuestEngine.check_combat_clear() end)
        end
    end

    -- Hunt: item pickup detection
    if s.hunt_step and s.hunt_step.loot and s.hunt_step.loot.items then
        for _, item in ipairs(s.hunt_step.loot.items) do
            if string.find(text, item, 1, true) and
               (string.find(text, "拿出") or string.find(text, "獲得")) then
                s.hunt_got_item = true
                mud.echo("[QuestEngine] Got item: " .. item)
            end
        end
    end

    -- Hunt: flee from non-target
    if s.non_target_combat and string.find(text, "你逃離了戰鬥") then
        s.non_target_combat = false
        s.phase = "finding"
        local MudExplorer = require_module("MudExplorer")
        MudExplorer.explore(s.hunt_explore_cb)
    end

    -- Hunt: recovery
    if s.phase == "fighting" then
        if string.find(text, "移動力不足") or string.find(text, "法力不足") then
            if not s.recovering then
                s.recovering = true
                mud.send("c ref")
                MudUtils.safe_timer(5.0, function()
                    if s.running and s.recovering then
                        s.recovering = false
                    end
                end)
            end
        end
        if string.find(text, "體力逐漸地恢復") then
            s.recovering = false
        end
    end
```

**Step 4: Run test to verify it passes**

Run: `lua scripts/tests/test_quest_engine.lua`
Expected: PASS

**Step 5: Commit**

```bash
git add scripts/modules/QuestEngine.lua scripts/tests/test_quest_engine.lua
git commit -m "feat: add QuestEngine hunt handler with combat, loot, backtrack"
```

---

### Task 4: Give, Say, Interact handlers

**Files:**
- Modify: `scripts/modules/QuestEngine.lua`
- Modify: `scripts/tests/test_quest_engine.lua`

**Step 1: Write the failing test**

```lua
describe("QuestEngine Simple Handlers", function()
    it("give handler should send 'gi <item> <npc>'", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("give_test", {
            steps = {
                {type="give", name="give_stone", item="stone", npc="king"},
            }
        })
        QuestEngine.run("give_test")

        local found = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "gi stone king" then found = true end
        end
        assert_equal(true, found, "Should send 'gi stone king'")
    end)

    it("say handler should send 'say <text>'", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("say_test", {
            steps = {
                {type="say", name="greet", text="hello world"},
            }
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
            steps = {
                {type="interact", name="push", cmd="push stone"},
            }
        })
        QuestEngine.run("interact_test")

        local found = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "push stone" then found = true end
        end
        assert_equal(true, found, "Should send 'push stone'")
    end)
end)
```

**Step 2: Run test to verify it fails**

Run: `lua scripts/tests/test_quest_engine.lua`
Expected: FAIL — handlers not defined

**Step 3: Write minimal implementation**

```lua
-- ===== Give Handler =====
QuestEngine.handlers["give"] = function(step, def)
    local s = QuestEngine.state
    s.phase = "giving"
    mud.send("gi " .. step.item .. " " .. step.npc)

    if step.expect then
        s.phase = "waiting_response"
        s.current_expect = step.expect
        MudUtils.safe_timer(10.0, function()
            if s.running and s.phase == "waiting_response" then
                mud.echo("[QuestEngine] Give timeout: " .. step.expect)
                QuestEngine.stop(false)
            end
        end)
    else
        -- Auto-advance after brief delay
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end

-- ===== Say Handler =====
QuestEngine.handlers["say"] = function(step, def)
    local s = QuestEngine.state

    -- Support both raw text and "say X" format
    local cmd = step.text
    if not cmd:match("^say ") and not cmd:match("^ta ") and not cmd:match("^talk ") then
        cmd = "say " .. cmd
    end

    mud.send(cmd)

    if step.expect then
        s.phase = "waiting_response"
        s.current_expect = step.expect
        MudUtils.safe_timer(10.0, function()
            if s.running and s.phase == "waiting_response" then
                mud.echo("[QuestEngine] Say timeout: " .. step.expect)
                QuestEngine.stop(false)
            end
        end)
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end

-- ===== Interact Handler =====
QuestEngine.handlers["interact"] = function(step, def)
    local s = QuestEngine.state

    -- Support single cmd or table of cmds
    if type(step.cmd) == "table" then
        for _, c in ipairs(step.cmd) do mud.send(c) end
    else
        mud.send(step.cmd)
    end

    if step.expect then
        s.phase = "waiting_response"
        s.current_expect = step.expect
        local timeout = step.timeout or 10.0
        MudUtils.safe_timer(timeout, function()
            if s.running and s.phase == "waiting_response" then
                if step.retry and (s.interact_retries or 0) < step.retry then
                    s.interact_retries = (s.interact_retries or 0) + 1
                    QuestEngine.execute_step() -- Retry
                else
                    mud.echo("[QuestEngine] Interact timeout: " .. step.expect)
                    QuestEngine.stop(false)
                end
            end
        end)
    else
        MudUtils.safe_timer(1.0, function() QuestEngine.advance() end)
    end
end
```

**Step 4: Run test to verify it passes**

Run: `lua scripts/tests/test_quest_engine.lua`
Expected: PASS

**Step 5: Commit**

```bash
git add scripts/modules/QuestEngine.lua scripts/tests/test_quest_engine.lua
git commit -m "feat: add QuestEngine give, say, interact handlers"
```

---

### Task 5: Recall and pre-navigation support

**Files:**
- Modify: `scripts/modules/QuestEngine.lua`

Quest definitions need `recall_cmd` for returning to town before navigation. Add support for:
- `def.recall_cmd` — sent before first navigate step
- `step.recall_first` — bool, send recall before this navigate step

**Step 1: Write the failing test**

```lua
describe("QuestEngine Recall Support", function()
    it("should send recall_cmd before first step if configured", function()
        mock.sent = {}
        MudUtils.run_id = 0
        MudUtils.callbacks = {}
        MudUtils.callback_id = 0

        QuestEngine.define("recall_test", {
            recall_cmd = "recall",
            steps = {
                {type="navigate", name="go", path={"s"}},
            }
        })
        QuestEngine.run("recall_test")

        -- First commands should include "wa" and "recall"
        local has_recall = false
        for _, cmd in ipairs(mock.sent) do
            if cmd == "recall" then has_recall = true end
        end
        assert_equal(true, has_recall, "Should send recall before navigate")
    end)
end)
```

**Step 2: Run test to verify it fails**

**Step 3: Implementation**

Modify `QuestEngine.run()` to add recall logic:

```lua
function QuestEngine.run(name)
    -- ... existing setup code ...

    -- Pre-navigation: recall if configured
    if def.recall_cmd then
        mud.send("wa")
        mud.send(def.recall_cmd)
        MudUtils.safe_timer(1.5, function()
            QuestEngine.execute_step()
        end)
    else
        QuestEngine.execute_step()
    end
end
```

**Step 4: Run test to verify it passes**

**Step 5: Commit**

```bash
git add scripts/modules/QuestEngine.lua scripts/tests/test_quest_engine.lua
git commit -m "feat: add QuestEngine recall support before first step"
```

---

### Task 6: Register in test_runner.lua

**Files:**
- Modify: `scripts/tests/test_runner.lua`

**Step 1: Add test file to runner**

Add `"scripts/tests/test_quest_engine.lua"` to the `test_files` table in `test_runner.lua`.

**Step 2: Run full test suite**

Run: `lua scripts/tests/test_runner.lua`
Expected: All tests PASS (including existing tests)

**Step 3: Commit**

```bash
git add scripts/tests/test_runner.lua
git commit -m "chore: register QuestEngine tests in test runner"
```

---

### Task 7: Port PokerQuest to QuestEngine

**Files:**
- Create: `scripts/poker_quest_v2.lua`

This is the validation step. Create a new file using QuestEngine's declarative format. Keep the old `poker_quest.lua` intact for comparison.

**Step 1: Write declarative PokerQuest**

```lua
-- PokerQuest v2 — Declarative version using QuestEngine
local QuestEngine = require("scripts.modules.QuestEngine")

QuestEngine.define("poker_quest", {
    recall_cmd = "recall",
    steps = {
        -- 1. Navigate to Poker Kingdom entrance (凍原山頂)
        {type="navigate", name="enter_poker_kingdom",
         path={
             "s", "s", "s", "s", "s", "s", "e", "e", "u", "u", "u",
             {cmd="u", id="20ce628eff898093aae8aea12ce15043ad2c599a254804579c1956afff2b4bef"},
         }},

        -- 2. Hunt Spade mobs for yellow stone
        {type="hunt", name="hunt_spade",
         target="小黑桃(spade)",
         attack_cmd="ear spade",
         max_laps=5,
         disable_open_doors=true,
         debug=true,
         loot={items={"stone"}, sac=true, loot_ground=true, fallback_blind=true}},

        -- 3. Navigate to Diamond King
        {type="navigate", name="deliver_to_king",
         path={
             "n", "n", "w", "w", "n", "n",
             {cmd="w", id="6faf86d9c11f591577f24cab47c7a2f29d980e9f424d85dcb97af994559b3f15"},
         }},

        -- 4. Give stone to Diamond King
        {type="interact", name="give_stone",
         cmd="gi stone king",
         expect="把 黃色石頭 給了"},

        -- 5. Navigate to Spade Queen
        {type="navigate", name="go_to_queen",
         path={
             "e", "n", "n", "e", "e",
             {cmd="n", id="68e309f3e2252bd02102e43fcc000c90a3551d36791d21974f54ebf92e929c21"},
         }},

        -- 6. Talk to Spade Queen
        {type="say", name="talk_queen",
         text="say goodmorning",
         expect="黑桃王后說道: 我可以告訴你是 'ireallywantleave'"},

        -- 7. Navigate to Heart Queen palace
        {type="navigate", name="go_to_palace",
         path={
             "s", "s", "s", "u", "u",
             {cmd="u", id="fd4a9da729fd717f3b7595f056e6a313877e48a865d17aaa4c73f69cbef078c6"},
         }},

        -- 8. Say the magic word to Heart Queen
        {type="say", name="say_password",
         text="say ireallywantleave",
         expect="紅心女王說道: 好吧 !我再試一試這咒語"},
    }
})

-- Entry point
_G.PokerQuestV2 = {
    start = function() QuestEngine.run("poker_quest") end,
    stop = function() QuestEngine.stop() end,
    status = function()
        local s = QuestEngine.state
        mud.echo("[PokerQuestV2] Running: " .. tostring(s.running))
        mud.echo("[PokerQuestV2] Step: " .. s.step_index .. " / Phase: " .. s.phase)
    end,
}

local MudUtils = require("scripts.modules.MudUtils")
MudUtils.show_script_usage("PokerQuestV2 (QuestEngine)", {
    "PokerQuestV2.start()   - Start quest",
    "PokerQuestV2.stop()    - Stop quest",
    "PokerQuestV2.status()  - Show status",
})

return _G.PokerQuestV2
```

**Step 2: Syntax check**

Run: `lua -c scripts/poker_quest_v2.lua` (or `luac -p`)

**Step 3: Manual integration test**

Load in MUD client session: `/lua dofile("scripts/poker_quest_v2.lua")`
Run: `/lua PokerQuestV2.start()`
Observe behavior matches original PokerQuest flow.

**Step 4: Commit**

```bash
git add scripts/poker_quest_v2.lua
git commit -m "feat: port PokerQuest to QuestEngine declarative format"
```

---

### Task 8: Handle Diamond King detection edge case

The original `poker_quest.lua` has special logic for detecting the Diamond King NPC (counting `king` occurrences to handle multiple kings in room). This needs to be supported in QuestEngine.

**Files:**
- Modify: `scripts/modules/QuestEngine.lua`

The `give` handler needs to support an optional `detect` field for NPC detection before giving:

```lua
-- In give handler, add detect logic:
{type="give", name="give_stone", item="stone", npc="king",
 detect={keyword="方塊國王(King)", count_keyword="國王(King)"}},
```

This is quest-specific complexity. Instead of bloating QuestEngine, use `interact` type with custom `cmd` that handles the detection inline:

```lua
-- Already handled in Task 7 by using:
{type="interact", name="give_stone", cmd="gi stone king", expect="把 黃色石頭 給了"},
```

If the simple `gi stone king` doesn't work because there are multiple kings, this can be addressed by adding a `pre_cmd` field to `interact`:

```lua
{type="interact", name="give_stone",
 cmd={"l", "gi stone king"},  -- look first, then give
 expect="把 黃色石頭 給了"},
```

**Decision:** Skip this task if the simple `gi stone king` works. Only add detect logic if live testing reveals issues. Mark as **conditional**.

---

### Task 9: End-to-end live test

**No code changes.** Run `PokerQuestV2.start()` on the MUD client and observe:

1. Recall and navigate to 凍原山頂
2. Explorer finds Spade mob
3. Combat heartbeat attacks, kill detected
4. Loot collects stone
5. Backtrack to explorer start
6. Navigate to Diamond King
7. Give stone
8. Navigate to Spade Queen, say goodmorning
9. Navigate to Heart Queen palace, say ireallywantleave
10. Quest completes

Fix any issues found, commit fixes.

---

## Summary

| Task | Description | Estimated Complexity |
|------|-------------|---------------------|
| 1 | Core: define, run, stop, advance | Simple |
| 2 | Navigate handler | Medium |
| 3 | Hunt handler (combat, loot, backtrack) | Complex |
| 4 | Give, Say, Interact handlers | Simple |
| 5 | Recall support | Simple |
| 6 | Register tests | Trivial |
| 7 | Port PokerQuest | Medium |
| 8 | King detection (conditional) | Simple |
| 9 | Live E2E test | Manual |

Total: 7 required tasks + 1 conditional + 1 manual test.
