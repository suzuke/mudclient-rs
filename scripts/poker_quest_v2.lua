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

QuestEngine.define("poker_quest", {
    recall_cmd = "recall",
    steps = {
        -- 1. Navigate to Poker Kingdom (via frozen mountain top)
        {type="navigate", name="enter_poker_kingdom",
         path={
             "s", "s", "s", "s", "s", "s", "e", "e", "u", "u", "u",
             {cmd="u", id="20ce628eff898093aae8aea12ce15043ad2c599a254804579c1956afff2b4bef"},
         }},

        -- 2. Hunt Spade mobs for yellow stone
        {type="hunt", name="hunt_spade",
         target="spade",
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
