-- ============================================================
-- SmurfQuest v2 — Declarative version using QuestEngine
-- ============================================================
-- Usage: /lua SmurfQuestV2.start()
-- Stop:  /lua SmurfQuestV2.stop()
-- Status:/lua SmurfQuestV2.status()
-- Loop:  /lua SmurfQuestV2.start_loop()
-- ============================================================

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("SmurfQuestV2 cannot load dependency: " .. name)
end

local QuestEngine = require_module("QuestEngine")
local MudUtils = require_module("MudUtils")

-- Combat constants
local COMBAT_V_THRESHOLD = 600   -- 大地之斬所需移動力
local COMBAT_MA_REFRESH = 40     -- refresh 所需法力

-- Custom: kill gargamel with report-based combat decisions
local function kill_gargamel(step, engine)
    local MudCombat = require_module("MudCombat")
    local s = engine.state
    s.phase = "fighting"
    s.combat_finished = false

    -- Check if gargamel is here via collect_response
    mud.collect_response("l", function(lines)
        _G._smurf_check_gargamel(lines)
    end)
end

_G._smurf_check_gargamel = function(lines)
    local s = QuestEngine.state
    if not s.running or s.combat_finished then return end

    lines = lines or {}
    local found = false
    for _, line in ipairs(lines) do
        -- Match NPC line specifically, not room description containing "賈不妙的城堡"
        if string.find(line, "賈不妙(Gargamel)", 1, true) then
            found = true
            break
        end
    end

    if found then
        mud.send("kill gargamel")
        combat_round(QuestEngine)
    else
        -- Gargamel not here, go south to find him
        mud.echo("[SmurfQuestV2] Gargamel not here, searching south...")
        mud.send("s")
        MudUtils.safe_timer(1.5, function()
            if not s.running or s.combat_finished then return end
            mud.collect_response("l", function(lines)
                _G._smurf_check_gargamel(lines)
            end)
        end)
    end
end

-- Combat round: collect_response("report") → parse → decide → next round
function combat_round(engine)
    local s = engine.state
    if not s.running or s.combat_finished then return end

    mud.collect_response("report", function(lines)
        _G._smurf_on_combat_report(lines)
    end)
end

_G._smurf_on_combat_report = function(lines)
    local s = QuestEngine.state
    if not s.running or s.combat_finished then return end

    lines = lines or {}
    local current_v, current_ma = 0, 0
    for _, line in ipairs(lines) do
        if line:find("你報告自己的狀況", 1, true) then
            local ma = line:match("(%d+)/%d+ 精神力")
            local v  = line:match("(%d+)/%d+ 移動力")
            if ma then current_ma = tonumber(ma) end
            if v  then current_v  = tonumber(v)  end
        end
    end

    if current_v >= COMBAT_V_THRESHOLD then
        mud.send("ear gargamel")
    elseif current_ma >= COMBAT_MA_REFRESH then
        mud.send("c ref")
    end

    MudUtils.safe_timer(4.0, function()
        if MudUtils.check_run(s.run_id) then
            combat_round(QuestEngine)
        end
    end)
end

-- Custom: get wand via MudLoot
local function get_wand(step, engine)
    local MudLoot = require_module("MudLoot")
    local s = engine.state

    MudUtils.safe_timer(3.0, function()
        if not s.running then return end
        MudLoot.process_loot({items = {"wand"}, sac = false}, function()
            if not s.running then return end
            engine.advance()
        end)
    end)
end

-- Custom: enter castle (unlock + open + walk)
local function enter_castle(step, engine)
    local MudNav = require_module("MudNav")
    local s = engine.state

    mud.send("un n")
    mud.send("op n")
    MudUtils.safe_timer(0.5, function()
        if not s.running then return end
        MudNav.walk("n", function()
            -- expect matching will advance via on_server_message
        end)
    end)
end

QuestEngine.define("smurf_quest", {
    recall_cmd = "recall",
    log_name = "smurf",
    precheck = {"papa", "gargamel"},
    on_fail = {path = "3w;3s;w", cmds = {"quests clear"}},

    steps = {
        -- 1. Navigate to village entrance
        {type="navigate", name="go_entrance",
         path="3w;3s;e;look painting;s;4e;4n;5n;2w;n",
         expect="通往賈不妙的城堡的小徑"},

        -- 2. Summon papa (first time)
        {type="summon", name="summon_papa_1",
         target="小精靈老爸", summon_cmd="c sum papa",
         max_retries=10, retry_delay=3.0, verify_delay=3.0},

        -- 3. Talk to papa
        {type="interact", name="talk_papa_yes",
         cmd="ta papa yes",
         expect="小精靈老爸 把 小鑰匙 給了你"},

        -- 4. Go to castle gate
        {type="navigate", name="go_castle_gate",
         path="n",
         expect="賈不妙的城堡外"},

        -- 5. Enter castle (unlock + open + walk)
        {type="custom", name="enter_castle",
         expect="他的大鍋放在房間的中央",
         fn=enter_castle},

        -- 6. Kill gargamel (report-based combat)
        {type="custom", name="kill_gargamel",
         fn=kill_gargamel},

        -- 7. Get wand from corpse
        {type="custom", name="get_wand",
         fn=get_wand},

        -- 8. Summon papa (second time, move south first)
        {type="summon", name="summon_papa_2",
         path="s",
         target="小精靈老爸", summon_cmd="c sum papa",
         max_retries=10, retry_delay=3.0, verify_delay=3.0},

        -- 9. Give wand to papa
        {type="interact", name="give_wand",
         cmd="gi wand papa",
         expect="小精靈老爸 把 粉紅藥劑 給了你"},
    }
})

-- Gargamel death detection — advance kill_gargamel step
MudUtils.register_hook("SmurfQuestV2", function(line, clean_line)
    if not QuestEngine.state.running then return end
    if QuestEngine.state.quest_name ~= "smurf_quest" then return end
    local text = clean_line or line

    -- Detect gargamel death
    if string.find(text, "賈不妙魂歸西天了", 1, true) then
        QuestEngine.state.combat_finished = true
        QuestEngine.advance()
    end
    -- Detect gargamel flee — pursue and re-engage
    local step = QuestEngine.quests["smurf_quest"].steps[QuestEngine.state.step_index]
    if step and step.name == "kill_gargamel" and not QuestEngine.state.combat_finished then
        if string.find(text, "賈不妙為了保命而不顧面子逃了", 1, true) or
           string.find(text, "賈不妙 往南邊離開了", 1, true) then
            MudUtils.safe_timer(1.0, function()
                if not QuestEngine.state.running or QuestEngine.state.combat_finished then return end
                mud.send("s")
                MudUtils.safe_timer(1.0, function()
                    if not QuestEngine.state.running or QuestEngine.state.combat_finished then return end
                    mud.send("kill gargamel")
                end)
            end)
        end
    end

    -- Non-combat flee detection (avoid unexpected fights)
    local step = QuestEngine.quests["smurf_quest"].steps[QuestEngine.state.step_index]
    if step and step.name ~= "kill_gargamel" then
        if string.find(text, "你現在正身陷戰鬥中", 1, true) then
            mud.send("flee")
        end
        if string.find(text, "不顧面子從戰鬥中逃了", 1, true) then
            QuestEngine.stop(false)
        end
    end
end)

-- Public API
_G.SmurfQuestV2 = {
    start = function() QuestEngine.run("smurf_quest") end,
    stop = function() QuestEngine.stop() end,
    start_loop = function()
        -- Enable loop mode on the quest definition
        QuestEngine.quests["smurf_quest"].loop = true
        QuestEngine.run("smurf_quest")
    end,
    status = function()
        local s = QuestEngine.state
        mud.echo("[SmurfQuestV2] Running: " .. tostring(s.running))
        if s.running then
            mud.echo("[SmurfQuestV2] Step: " .. s.step_index .. " Phase: " .. tostring(s.phase))
        end
    end,
}

MudUtils.show_script_usage("SmurfQuestV2 (QuestEngine)", {
    "SmurfQuestV2.start()      - Start quest",
    "SmurfQuestV2.stop()       - Stop quest",
    "SmurfQuestV2.start_loop() - Start with loop mode",
    "SmurfQuestV2.status()     - Show status",
})

return _G.SmurfQuestV2
