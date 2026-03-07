-- ============================================================
-- PokerQuest v2 — Declarative version using QuestEngine
-- ============================================================
-- Usage: /lua PokerQuestV2.start()
-- Stop:  /lua PokerQuestV2.stop()
-- Status:/lua PokerQuestV2.status()
-- ============================================================

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("PokerQuestV2 cannot load dependency: " .. name)
end

local QuestEngine = require_module("QuestEngine")
local MudUtils = require_module("MudUtils")

-- Custom: wait for sleep teleport at frozen mountain top
local function wait_for_teleport(step, engine)
    QuestEngine.set_expect("紙路", 30.0, "Teleport")
end

-- Hook: detect wake-up and send look to trigger expect
MudUtils.register_hook("PokerQuestV2", function(line, clean_line)
    if not QuestEngine.state.running then return end
    if QuestEngine.state.quest_name ~= "poker_quest" then return end
    local text = clean_line or line
    local step = QuestEngine.quests["poker_quest"].steps[QuestEngine.state.step_index]
    if step and step.name == "enter_poker_kingdom" then
        if string.find(text, "你從睡夢中醒來並站了起來", 1, true) then
            MudUtils.safe_timer(1.0, function()
                if QuestEngine.state.running then
                    mud.send("l")
                end
            end)
        end
    end
end)

QuestEngine.define("poker_quest", {
    recall_cmd = "recall",
    log_name = "poker",
    steps = {
        -- 1. Navigate to frozen mountain top
        {type="navigate", name="go_mountain_top",
         path="6s;2e;4u"},

        -- 2. Wait for sleep teleport to poker kingdom
        {type="custom", name="enter_poker_kingdom",
         expect="紙路",
         fn=wait_for_teleport},

        -- 2. Hunt Spade mobs for yellow stone
        {type="hunt", name="hunt_spade",
         target="spade",
         explorer_target="(spade)正站在",
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
         expect="給了"},

        -- 5. Navigate to Spade Queen
        {type="navigate", name="go_to_queen",
         path={
             "e", "n", "n", "e", "e",
             {cmd="n", id="68e309f3e2252bd02102e43fcc000c90a3551d36791d21974f54ebf92e929c21"},
         }},

        -- 6. Talk to Spade Queen
        {type="say", name="talk_queen",
         text="say goodmorning",
         expect="黑桃王后說道"},

        -- 7. Navigate to Heart Queen palace
        {type="navigate", name="go_to_palace",
         path={
             "s", "s", "s", "u", "u",
             {cmd="u", id="fd4a9da729fd717f3b7595f056e6a313877e48a865d17aaa4c73f69cbef078c6"},
         }},

        -- 8. Say password to Heart Queen
        {type="say", name="say_password",
         text="say ireallywantleave",
         expect="紅心女王說道"},
    }
})

-- Public API
_G.PokerQuestV2 = {
    start = function() QuestEngine.run("poker_quest") end,
    stop = function() QuestEngine.stop() end,
    status = function()
        local s = QuestEngine.state
        mud.echo("[PokerQuestV2] Running: " .. tostring(s.running))
        if s.running then
            mud.echo("[PokerQuestV2] Step: " .. s.step_index .. " Phase: " .. s.phase)
            if s.hunt_kills then
                mud.echo("[PokerQuestV2] Kills: " .. s.hunt_kills)
            end
        end
    end,
}

MudUtils.show_script_usage("PokerQuestV2 (QuestEngine)", {
    "PokerQuestV2.start()   - Start quest",
    "PokerQuestV2.stop()    - Stop quest",
    "PokerQuestV2.status()  - Show status",
})

return _G.PokerQuestV2
