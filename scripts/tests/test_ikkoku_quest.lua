-- Test IkkokuQuest
local MockMud = require("scripts/tests/mock_mud")
local MudUtils = require("scripts/modules/MudUtils")

_G.describe("IkkokuQuest", function()
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
        -- We will load IkkokuQuest inside the test to simulate fresh start
    end

    _G.it("should integration test start flow", function()
        setup()
        local IkkokuQuest = require("scripts.ikkoku_quest")
        
        -- Mock Config
        IkkokuQuest.config.entry_path = "n;e"
        
        -- Start
        IkkokuQuest.start()
        
        -- Check initial sequence
        assert_equal("recall", mock.sent[1], "Should send recall first")
        
        -- Advance timer for recall
        mock:tick(1.6)
        
        -- Should start walking entry path "n;e"
        assert_equal("n", mock.sent[2])
        
        -- Need to use the SAME MudNav instance
        local MudNav = require("scripts.modules.MudNav")
        MudNav.config.debug = true -- Enable debug
        MudNav.on_server_message("[Exits: North South]")
        mock:tick(0.6)
        
        assert_equal("e", mock.sent[3])
        
        -- After entry path, it should transition to process_step
        -- MudNav finishes -> callback (on_entered) -> wa -> process_step
        MudNav.on_server_message("[Exits: East West]")
        mock:tick(0.6)
        
        -- Walk Complete.
        -- on_entered sends "enter ikkoku" first
        -- Then timer 1.0s -> "wa"
        
        -- Check for "enter ikkoku"
        assert_equal("enter ikkoku", mock.sent[4])
        
        -- Advance timer for scene switch (1.0s)
        mock:tick(1.1)
        
        -- Check for "wa"
        assert_equal("wa", mock.sent[5])
        
        -- After "wa", process_step(1) starts.
        -- Step 1 "wait_kyokoo" path="n;op n;n;w;op n;n"
        
        mock:tick(0.6) -- Wait for MudNav path parsing/start
        
        -- 1. n (Movement)
        assert_equal("n", mock.sent[6])
        MudNav.on_server_message("[Exits: North South]")
        mock:tick(0.6)
        
        -- 2. op n (Action)
        assert_equal("op n", mock.sent[7])
        MudNav.on_server_message("OK.")
        mock:tick(0.6)
        
        -- 3. n (Movement)
        assert_equal("n", mock.sent[8])
        MudNav.on_server_message("[Exits: North South]")
        mock:tick(0.6)
        
        -- 4. w (Movement)
        assert_equal("w", mock.sent[9])
        MudNav.on_server_message("[Exits: East West]")
        mock:tick(0.6)
        
        -- 5. op n (Action)
        assert_equal("op n", mock.sent[10])
        MudNav.on_server_message("OK.")
        mock:tick(0.6)
        
        -- 6. n (Movement)
        assert_equal("n", mock.sent[11])
        MudNav.on_server_message("[Exits: South]") -- Manager room
        mock:tick(0.6)
        
        -- Walk Complete. Callback -> check_arrival
        -- It should set phase="waiting_for_mob" and send "l"
        mock:tick(0.6) -- Extra tick for check_arrival timer
        
        -- Should have sent "l"
        assert_equal("l", mock.sent[12])
        
        -- Simulate "Kyokoo is here" (response to "l")
        IkkokuQuest.on_server_message("Kyokoo is here.", "Kyokoo is here.")
        
        local sent_len = #mock.sent
        -- Ensure we actually sent more commands (talk)
        -- sent[13] should be talk
        assert(sent_len >= 13, "Should have sent talk commands")
        assert_equal("talk kyokoo otonashi", mock.sent[sent_len-1])
        assert_equal("talk kyokoo yes", mock.sent[sent_len])
        
    end)

    -- "should handle go_keeper fallback" removed as it tests unreachable code (go_keeper is fixed path now)
end)

