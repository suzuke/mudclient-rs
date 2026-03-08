-- ============================================================
-- IkkokuQuest v2 — Declarative version using QuestEngine
-- ============================================================
-- Usage: /lua IkkokuQuestV2.start()
-- Stop:  /lua IkkokuQuestV2.stop()
-- Status:/lua IkkokuQuestV2.status()
-- ============================================================

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("IkkokuQuestV2 cannot load dependency: " .. name)
end

local QuestEngine = require_module("QuestEngine")
local MudUtils = require_module("MudUtils")

-- Yotsuya search loop: cycles gap → room4 → room5, trying "talk yotsuya godai" at each
local function yotsuya_search(step, engine)
    local s = engine.state

    local function try_talk()
        if not s.running then return end
        mud.send("talk yotsuya godai")
        s.phase = "waiting_response"
        s.current_expect = step.expect
    end

    local function loop()
        if not s.running then return end

        -- 1. Try in gap (current position after "squeeze")
        try_talk()

        -- 2. Go to room 4
        MudUtils.safe_timer(1.5, function()
            if not s.running or s.phase ~= "waiting_response" then return end
            mud.send("squeeze east")
            MudUtils.safe_timer(1.2, function()
                if not s.running or s.phase ~= "waiting_response" then return end
                try_talk()
                mud.send("squeeze") -- back to gap

                -- 3. Go to room 5
                MudUtils.safe_timer(1.5, function()
                    if not s.running or s.phase ~= "waiting_response" then return end
                    mud.send("squeeze west")
                    MudUtils.safe_timer(1.2, function()
                        if not s.running or s.phase ~= "waiting_response" then return end
                        try_talk()
                        mud.send("squeeze") -- back to gap

                        -- 4. Loop
                        MudUtils.safe_timer(2.0, loop)
                    end)
                end)
            end)
        end)
    end

    loop()
end

QuestEngine.define("ikkoku_quest", {
    recall_cmd = "recall",
    log_name = "ikkoku",
    precheck = {"otonashi", "kyokoo", "godai", "yukari", "yotsuya", "akemi", "keeper"},
    on_fail = {path = "3w;3s;w", cmds = {"wa", "quests clear"}},

    -- Entry: after recall, walk to ikkoku entrance then enter
    entry_path = "6w;3n",
    entry_cmd = "enter ikkoku",

    steps = {
        -- 1. Navigate to entrance and enter
        {type="navigate", name="enter_ikkoku",
         path="6w;3n",
         cmds={"enter ikkoku"}},

        -- 2. Wait for Kyokoo, talk to her
        {type="wait_for_mob", name="wait_kyokoo",
         path="n;op n;n;w;op n;n",
         target="kyokoo", target_alias="音無響子(Kyokoo)",
         cmds={"talk kyokoo otonashi", "talk kyokoo yes"},
         expect="看能不能說服他進來"},

        -- 3. Find Otonashi (summon only — he's outside ikkoku)
        {type="summon", name="find_otonashi_1",
         path="op s;s;e",
         target="響子的爸爸", summon_cmd="cast 'summon' otonashi",
         max_retries=10, verify_delay=2.0,
         cmds={"talk otonashi kyokoo"},
         expect="不要....叫響子出來見我..!!"},

        -- 4. Wait for Kyokoo again
        {type="wait_for_mob", name="find_kyokoo_2",
         path="w;n",
         target="kyokoo", target_alias="音無響子(Kyokoo)",
         cmds={"talk kyokoo otonashi"},
         expect="也許五代有辦法，你去問他看看吧..."},

        -- 5. Wait for Godai and talk
        {type="wait_for_mob", name="find_godai_1",
         path="open s;s;3e;n;u;s;2w;op n;n",
         target="godai", target_alias="五代裕作(Yusaku Godai)",
         cmds={"talk godai otonashi"},
         expect="也許我奶奶有辦法吧....你去問看看吧.."},

        -- 6. Wait for Yukari and talk
        {type="wait_for_mob", name="find_yukari",
         path="op s;s;2e;n;d;s;3w;n",
         target="yukari", target_alias="五代由加莉(Yukari)",
         cmds={"talk yukari godai", "talk yukari otonashi"},
         expect="五代由加莉 把 錦囊 給了你."},

        -- 7. Wait for Godai and give bag
        {type="wait_for_mob", name="find_godai_2",
         path="open s;s;3e;n;u;s;2w;op n;n",
         target="godai", target_alias="五代裕作(Yusaku Godai)",
         cmds={"gi bag godai"},
         expect="我奶奶說可以試著找四谷先生幫忙...不過四谷是個很怪的人喔.."},

        -- 8. Yotsuya search loop (custom)
        {type="navigate", name="go_to_gap",
         path="squeeze"},

        {type="custom", name="talk_yotsuya",
         expect="找朱美比較好解決",
         fn=yotsuya_search},

        -- 9. Wait for Akemi and talk (akemi cannot be summoned)
        {type="wait_for_mob", name="find_akemi_1",
         path="squeeze east;s;e;op n;n",
         target="akemi", target_alias="朱美小姐(Akemi)",
         cmds={"talk akemi yotsuya"},
         expect="那麼你只要給我一瓶茶茶丸的白酒"},

        -- 10. Talk to keeper (may be outside or inside bar)
        -- If keeper not found outside within 15s, enter chachamaru directly
        {type="wait_for_mob", name="go_keeper",
         path="op s;s;e;n;d;s;2w;op s;s;s;push door;n;w",
         target="keeper", target_alias="一個年約四十的老闆(keeper)",
         cmds={"talk keeper akemi"},
         expect="好...你跟我來一下...",
         wait_timeout=15, timeout_cmds={"enter chachamaru"}},

        -- 11. Enter chachamaru and get wine
        {type="wait_for_mob", name="chachamaru",
         path="enter chachamaru",
         target="keeper", target_alias="一個年約四十的老闆(keeper)",
         cmds={"talk keeper akemi"},
         expect="茶茶丸的老闆 把 白酒 給了你"},

        -- 12. Wait for Akemi and give wine (akemi cannot be summoned)
        {type="wait_for_mob", name="find_akemi_2",
         path="push door;e;s;enter ikkoku;n;op n;n;2e;n;u;s;w;op n;n",
         target="akemi", target_alias="朱美小姐(Akemi)",
         cmds={"gi wine akemi"},
         expect="你把 白酒 給了 朱美."},

        -- 13. Final: summon Otonashi again
        {type="summon", name="find_otonashi_2",
         path="op s;s;e;n;d;s;w;w",
         target="響子的爸爸", summon_cmd="cast 'summon' otonashi",
         max_retries=10, verify_delay=2.0,
         cmds={"talk otonashi kyokoo"},
         expect="為了感謝你的幫助，這個戒指就送給你吧!!"},
    }
})

-- Public API
_G.IkkokuQuestV2 = {
    start = function() QuestEngine.run("ikkoku_quest") end,
    stop = function() QuestEngine.stop() end,
    status = function()
        local s = QuestEngine.state
        mud.echo("[IkkokuQuestV2] Running: " .. tostring(s.running))
        if s.running then
            mud.echo("[IkkokuQuestV2] Step: " .. s.step_index .. " Phase: " .. tostring(s.phase))
        end
    end,
}

MudUtils.show_script_usage("IkkokuQuestV2 (QuestEngine)", {
    "IkkokuQuestV2.start()   - Start quest",
    "IkkokuQuestV2.stop()    - Stop quest",
    "IkkokuQuestV2.status()  - Show status",
})

return _G.IkkokuQuestV2
