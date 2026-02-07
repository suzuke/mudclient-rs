-- mudclient-rs 範例腳本
-- 這是一個 Lua 腳本範例，展示如何使用 mud 物件進行自動化操作。

-- 1. 基本輸出
mud.echo("--------------------------------------------------")
mud.echo("腳本載入成功！")
mud.echo("--------------------------------------------------")

-- 2. 變數操作
-- 讀取變數 (假設已有設定，若無則為 nil)
local target = mud.variables["target"]
if not target then
    target = "unknown"
    mud.echo("尚未設定目標，預設為: " .. target)
else
    mud.echo("當前目標: " .. target)
end

-- 設定新變數
mud.variables["last_script_run"] = os.date()

-- 3. 發送指令
-- 定義一個簡單的戰鬥準備函數
function prepare_feed()
    mud.send("get bread")
    mud.send("eat bread")
    mud.send("drink water")
end

-- 4. 條件判斷與延遲
-- 假設這是由一個觸發器調用的，captures 包含了血量資訊
-- 模擬情境：觸發器捕捉到 "HP: 50/100"
-- 測試時可以手動設定 captures
if captures and captures[1] then
    local hp = tonumber(captures[1])
    if hp < 50 then
        mud.echo("⚠️ 血量過低 (" .. hp .. ")，緊急治療！")
        mud.send("cast 'heal'")
        
        -- 延遲 3 秒後再檢查一次
        mud.timer(3.0, "mud.send('score')")
    end
end

-- 5. 視窗分流
-- 將特定訊息發送到 'chat' 視窗
mud.window("chat", "[腳本]這是一個測試訊息，發送到 chat 視窗")

-- 6. 動態啟用/禁用觸發器
-- 例如：進入戰鬥模式時開啟自動喝水
function combat_mode(enable)
    if enable then
        mud.enable_trigger("auto_potion", true)
        mud.echo("⚔️ 戰鬥模式開啟")
    else
        mud.enable_trigger("auto_potion", false)
        mud.echo("🛡️ 戰鬥模式結束")
    end
end

-- 7. 循環計時器 (Ticker)
-- 實作一個每 N 秒執行一次的循環
function start_ticker(seconds, command)
    mud.echo("啟動循環計時器: 每 " .. seconds .. " 秒執行 '" .. command .. "'")
    
    -- 定義一個遞歸函數來實現循環
    local function loop_action()
        -- 執行指令
        mud.send(command)
        
        -- 設定下一次執行 (遞歸調用)
        -- 注意：這裡我們動態生成一行 Lua 代碼來回調 loop_action
        -- 由於 mud.timer 接受的是字串形式的 Lua 代碼，我們需要用一個全域變數或函數來讓它調用
        -- 為了簡單起見，這裡演示最基本的方法：使用 mud.timer 重複執行 mud.send
        
        local code = string.format("mud.send('%s'); mud.timer(%f, [[ mud.send('%s'); mud.echo('Ticker tick.'); ]])", command, seconds, command)
        
        -- 更進階的做法是將 loop 函數設為全局，然後回調它
        -- 這裡我們單純展示發送指令
    end
    
    -- 啟動第一次
    mud.timer(seconds, string.format("mud.send('%s'); mud.echo(' ticker executed.');", command))
    
    -- 提示：要做真正的無限循環 ticker，建議使用遞歸呼叫全域函數的方式
    -- 例如：
    -- _G.my_ticker_enabled = true
    -- function _G.my_ticker_loop()
    --     if _G.my_ticker_enabled then
    --         mud.send("look")
    --         mud.timer(5.0, "_G.my_ticker_loop()")
    --     end
    -- end
    -- _G.my_ticker_loop()
end

-- 範例：啟動一個 60 秒的循環 (解除註解使用)
-- _G.my_ticker_enabled = true
-- function _G.keep_alive()
--     if _G.my_ticker_enabled then
--         mud.send("score") -- 防止斷線
--         mud.timer(60.0, "_G.keep_alive()")
--     end
-- end
-- _G.keep_alive()

-- 結束
mud.echo("範例腳本執行完畢。")
