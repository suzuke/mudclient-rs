-- Integration Test for IkkokuQuest using Real Logs
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")

_G.describe("IkkokuQuest Integration (Real Logs)", function()
    local mock
    
    local function setup()
        mock = MockMud.new()
        _G.mud = mock
        
        -- Clean package loaded
        for k, _ in pairs(package.loaded) do
            if k:match("MudUtils") or k:match("MudNav") or k:match("MudExplorer") or k:match("ikkoku_quest") then
                package.loaded[k] = nil
            end
        end
        
        require("scripts.modules.MudUtils")
        require("scripts.modules.MudNav")
        require("scripts.modules.MudExplorer")
    end

    _G.it("should follow the path from logs and handle waiting for Kyokoo", function()
        setup()
        local IkkokuQuest = require("scripts.ikkoku_quest")
        local MudNav = require("scripts.modules.MudNav")
        
        -- Config from logs: logs/ikkoku_1771085098.txt
        -- Entry entry_path seems to be "6w;3n" based on logs
        IkkokuQuest.config.entry_path = "6w;3n"
        IkkokuQuest.config.debug = true
        
        -- Start
        IkkokuQuest.start()
        
        -- [Log Line 35] [MudNav] Start Walk: 6w;3n (9 steps)
        assert_equal("recall", mock.sent[1], "Should recall")
        mock:tick(1.6) -- Advance for recall
        
        -- Verify Walk Sequence (6w; 3n)
        -- 1. w
        assert_equal("w", mock.sent[2])
        MudNav.on_server_message("和平大道")
        MudNav.on_server_message("[出口: 北 東 南 西 上]")
        mock:tick(0.6)
        
        -- 2. w
        assert_equal("w", mock.sent[3])
        MudNav.on_server_message("和平大道")
        MudNav.on_server_message("[出口: 北 東 南 西]")
        mock:tick(0.6)
        
        -- 3. w
        assert_equal("w", mock.sent[4])
        MudNav.on_server_message("和平大道")
        MudNav.on_server_message("[出口: 北 東 南 西 下]")
        mock:tick(0.6)
        
        -- 4. w
        assert_equal("w", mock.sent[5])
        MudNav.on_server_message("和平大道")
        MudNav.on_server_message("[出口: 東 南 西]")
        mock:tick(0.6)
        
        -- 5. w
        assert_equal("w", mock.sent[6])
        MudNav.on_server_message("風企公路起點")
        MudNav.on_server_message("[出口: 東 西]")
        mock:tick(0.6)
        
        -- 6. w
        assert_equal("w", mock.sent[7])
        MudNav.on_server_message("風企公路三叉路口")
        MudNav.on_server_message("[出口: 北 東 西]")
        mock:tick(0.6)
        
        -- 7. n
        assert_equal("n", mock.sent[8])
        MudNav.on_server_message("北環小道")
        MudNav.on_server_message("[出口: 北 南]")
        mock:tick(0.6)
        
        -- 8. n
        assert_equal("n", mock.sent[9])
        MudNav.on_server_message("北環小道")
        MudNav.on_server_message("[出口: 北 南 西]")
        mock:tick(0.6)
        
        -- 9. n
        assert_equal("n", mock.sent[10])
        MudNav.on_server_message("北環小道")
        MudNav.on_server_message("[出口: 北 南 西]")
        mock:tick(0.6)
        
        -- Walk Complete. Callback on_entered.
        -- [Log Line 268] [IkkokuQuest] ✅ 到達入口，進入一刻館...
        -- Sends "enter ikkoku"
        assert_equal("enter ikkoku", mock.sent[11])
        
        -- Timer 1.0s -> "wa" -> process_step
        mock:tick(1.1)
        assert_equal("wa", mock.sent[12])
        
        -- process_step(1)
        -- Step 1: "wait_kyokoo". Path: "n;op n;n;w;op n;n"
        mock:tick(0.6) -- Wait for MudNav path parsing
        
        -- 1. n
        assert_equal("n", mock.sent[13])
        MudNav.on_server_message("北環小道")
        MudNav.on_server_message("[出口: 北 南 西]")
        mock:tick(0.6)
        
        -- 2. op n
        assert_equal("op n", mock.sent[14])
        MudNav.on_server_message("這個方向的門老早就是開著的了.") -- From log line 348
        mock:tick(0.6)
        
        -- 3. n
        assert_equal("n", mock.sent[15])
        MudNav.on_server_message("玄關")
        MudNav.on_server_message("[出口: 東 南 西]")
        mock:tick(0.6)
        
        -- 4. w
        assert_equal("w", mock.sent[16])
        MudNav.on_server_message("走廊")
        MudNav.on_server_message("[出口: 北 東]")
        mock:tick(0.6)
        
        -- 5. op n
        assert_equal("op n", mock.sent[17])
        MudNav.on_server_message("這個方向的門老早就是開著的了.") -- From log line 414
        mock:tick(0.6)
        
        -- 6. n
        assert_equal("n", mock.sent[18])
        MudNav.on_server_message("管理人室")
        MudNav.on_server_message("這裡是響子的房間。左側是料理檯，上面擺滿了響子剛買的蔬果，似乎準備要做")
        MudNav.on_server_message("一道豐盛的料理。右側是響子的衣櫃，是一個很古典的衣櫃，響子看來十分的珍")
        MudNav.on_server_message("惜它。中間放了一個桌子，上頭擺了五代送的玫瑰，桌子下有一個日式暖爐。北")
        MudNav.on_server_message("邊有個門通往後院。")
        MudNav.on_server_message("[出口: 南]")
        
        -- Important: In the log, Kyokoo is NOT here initially.
        -- Log Line 451: 五代的奶奶--五代由加莉(Yukari)正站在這兒。
        MudNav.on_server_message("五代的奶奶--五代由加莉(Yukari)正站在這兒。")
        mock:tick(0.6)
        
        -- Walk Complete. Callback check_arrival.
        mock:tick(0.6) 

        -- Should look
        assert_equal("l", mock.sent[19])
        
        -- Simulate "l" output from log (Kyokoo missing)
        IkkokuQuest.on_server_message("管理人室", "管理人室")
        IkkokuQuest.on_server_message("五代的奶奶--五代由加莉(Yukari)正站在這兒。", "五代的奶奶--五代由加莉(Yukari)正站在這兒。")
        
        -- Should enter wait loop.
        assert_equal("waiting_for_mob", IkkokuQuest.state.phase)
        
        -- Wait 5 seconds (Wait loop timer)
        mock:tick(5.1)
        
        -- Should have sent "l" again from wait loop
        assert_equal("l", mock.sent[20], "Should re-check room")
        
        -- Now simulating Kyokoo appears!
        -- "Kyokoo is here." (Fake it as if it appeared in room desc or just arrived)
        IkkokuQuest.on_server_message("音無響子(Kyokoo)正站在這兒。", "音無響子(Kyokoo)正站在這兒。")
        
        -- Should match target "kyokoo" or "音無響子"
        -- Step cmds: "talk kyokoo otonashi", "talk kyokoo yes"
        
        local sent_len = #mock.sent
        assert_equal("talk kyokoo otonashi", mock.sent[sent_len-1])
        assert_equal("talk kyokoo yes", mock.sent[sent_len])
        
        -- Phase should update
        -- Step 1 expect="看能不能說服他進來" -> waiting_response
        assert_equal("waiting_response", IkkokuQuest.state.phase)
        
    end)
end)
