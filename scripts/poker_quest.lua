-- ============================================================
-- PokerQuest - 撲克王國解謎任務自動腳本 (Refactored)
-- ============================================================
-- 使用: /lua PokerQuest.start()
-- 停止: /lua PokerQuest.stop()
-- 狀態: /lua PokerQuest.status()
-- ============================================================

_G.PokerQuest = _G.PokerQuest or {}

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("PokerQuest cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")
local MudNav = require_module("MudNav")
local MudExplorer = require_module("MudExplorer")
local MudCombat = require_module("MudCombat")
local MudLoot = require_module("MudLoot")

-- Hot-reload Modules if needed (Consistency check)
if not MudUtils.show_script_usage then
    package.loaded["scripts.modules.MudUtils"] = nil
    MudUtils = require_module("MudUtils")
end

-- ===== 設定 =====
_G.PokerQuest.config = {
    entry_path = {
        {cmd="s", id="e9ca433ae6bf286bb394cb20797788aa82752ce41c23db9ac8adf1090d1841a0"},
        {cmd="s", id="4cc194ee9b17a4babe56a7d3fd09f6b91d12c53815b2f41a2374d91e55cab7d5"},
        {cmd="s", id="50e9b14484e851fe5f89c254b0d7323d801812d9bb642b60051e369406f612f4"},
        {cmd="s", id="f5b5481ddf5d80145ae59baaf99e5f6855e54329dbc7b8a41edab66051b5ff21"},
        {cmd="s", id="ef467c3f2d77df0093f7ef05f12bf37feaee13a039a52d8d7f733074dec0d43c"},
        {cmd="s", id="d8253dcc9aea4a71b5d879ef074cd3f1c5088329ce2c144bb9b6e4c0cec6e618"},
        {cmd="e", id="341107bbc2a2ce7862c26903d44750da6380667b1cb2b0358a9c453b085e45ff"},
        {cmd="e", id="f2260278c72687e26cf1ac6a8c4ed66c47e43eb35602296098276e0f82fccf15"},
        {cmd="u", id="52250d8b7fa01f3399c655689e69ac5ac309dc0b9b566832335277d7f372e82a"},
        {cmd="u", id="52250d8b7fa01f3399c655689e69ac5ac309dc0b9b566832335277d7f372e82a"},
        {cmd="u", id="3042cd4b665e86ad0a464868444932c797406b3d281bc58839a46679814801a9"},
        {cmd="u", id="b85bde0b8311050d5230c05959bb91b6a26b98f30a653b62b127d61051ae9a96"},
    },
    deliver_path = {
            {cmd="n", id="edb72730d4dbe6a0e94d7a198100c6062911f27bbd353996a03327f290b93549"},
    {cmd="n", id="599a2b3c6ae523baa16b9b0e003f37f630d8b12ab59df0159c8e007a1dd4eb35"},
    {cmd="w", id="8a19a29ac0bafbec608252709ac6d401cb4335c2939eacf2ed42cc594c0debe1"},
    {cmd="w", id="bbf885d5efc376574ece981fa6c722ce81cb885831aeabecc48488f95d1625ff"},
    {cmd="n", id="1f500fee6b93223e3fb05f90d5d736a3afe10945575ed0e405345ea4e23e6eaf"},
    {cmd="n", id="599a2b3c6ae523baa16b9b0e003f37f630d8b12ab59df0159c8e007a1dd4eb35"},
    {cmd="w", id="89a825185d2fa97ccc251c63d2183726a64d085969240701f3789f50ec9db535"},
    },
    attack_cmd = "ear spade",
    debug = false,
    max_laps = 5,
}

-- ===== 任務步驟 =====
local QUEST_STEPS = {
    {name="explore_spade",  handler="do_explore_spade", expect="success_or_fail", next="deliver_stone"},
    {name="deliver_stone",  path=_G.PokerQuest.config.deliver_path, handler="do_deliver_stone", expect="把 黃色石頭 給了", next="talk_queen"},
    -- Use Queen's response confirm success
    {name="talk_queen",     path="e;2n;2e;n", cmds={"say goodmorning"}, expect="黑桃王后說道: 我可以告訴你是 'ireallywantleave'", next="go_palace"},
    -- Use Heart Queen response or generic say confirm
    {name="go_palace",      path="3s;3u", cmds={"say ireallywantleave"}, expect="紅心女王說道: 好吧 !我再試一試這咒語", next="done"},
}

local STEP_BY_NAME = {}
for i, step in ipairs(QUEST_STEPS) do STEP_BY_NAME[step.name] = i end

-- ===== 狀態 =====
_G.PokerQuest.state = {
    running = false,
    run_id = 0,
    step_index = 0,
    phase = "idle",
    kills = 0,
    got_stone = false,
    finding = false, -- MudExplorer take over
    corpse_offset = 0,
}

local function check_run(rid)
    return rid == _G.PokerQuest.state.run_id
end

function _G.PokerQuest.echo(msg)
    mud.echo("[PokerQuest] " .. msg)
end

-- ===== 公開 API =====

function _G.PokerQuest.start()
    if _G.PokerQuest.state.running then
        _G.PokerQuest.echo("⚠️ 任務已在執行中")
        return
    end

    _G.PokerQuest.state.running = true
    _G.PokerQuest.state.run_id = MudUtils.get_new_run_id()
    _G.PokerQuest.state.step_index = 1
    _G.PokerQuest.state.phase = "starting"
    _G.PokerQuest.state.kills = 0
    _G.PokerQuest.state.got_stone = false
    _G.PokerQuest.state.finding = false
    
    MudUtils.start_log("poker")
    MudUtils.register_quest("PokerQuest", _G.PokerQuest.stop)
    
    _G.PokerQuest.echo("🚀 啟動撲克王國任務！")
    
    -- Recall & Enter
    MudUtils.send_cmds("wa;recall")
    MudUtils.safe_timer(1.5, _G.PokerQuest.enter_area)
end

function _G.PokerQuest.enter_area(rid)
    if not check_run(rid) then return end
    _G.PokerQuest.echo("🚶 前往撲克王國...")
    mud.send("wa")
    MudNav.walk(_G.PokerQuest.config.entry_path, _G.PokerQuest.process_step)
end

function _G.PokerQuest.stop(is_success)
    local s = _G.PokerQuest.state
    if not s.running then return end
    
    s.running = false
    s.phase = "stopped"
    
    MudUtils.get_new_run_id() -- Invalidate pending
    MudExplorer.stop()
    MudNav.reset()
    MudUtils.stop_log()

    if is_success then
        _G.PokerQuest.echo("🛑 任務圓滿完成！(擊殺: " .. s.kills .. ")")
        MudUtils.send_cmds("wa;recall")
    else
        _G.PokerQuest.echo("🛑 任務中止，Recall 回城...")
        MudUtils.send_cmds("wa;recall")
    end
end

function _G.PokerQuest.status()
    local s = _G.PokerQuest.state
    _G.PokerQuest.echo("📊 狀態: " .. (s.running and "執行中" or "停止"))
    local step = QUEST_STEPS[s.step_index]
    _G.PokerQuest.echo("   步驟: " .. (step and step.name or "N/A"))
    _G.PokerQuest.echo("   擊殺: " .. s.kills)
    _G.PokerQuest.echo("   石頭: " .. (s.got_stone and "✅" or "❌"))
end

function _G.PokerQuest.reload()
    package.loaded["scripts.poker_quest"] = nil
    require("scripts.poker_quest")
    _G.PokerQuest.echo("♻️ 腳本已重新載入")
end

-- ===== 核心流程邏輯 =====

function _G.PokerQuest.process_step(rid)
    if type(rid) ~= "number" then rid = _G.PokerQuest.state.run_id end -- Handle being called without rid (e.g. from Nav callback)
    if not check_run(rid) then return end
    
    local s = _G.PokerQuest.state
    local step = QUEST_STEPS[s.step_index]
    
    if not step then
        _G.PokerQuest.echo("🎉 任務完成！")
        _G.PokerQuest.stop(true)
        return
    end

    _G.PokerQuest.echo("📍 步驟 [" .. s.step_index .. "]: " .. step.name)

    -- 路徑移動模式
    if step.path then
        s.phase = "navigating"
        MudNav.walk(step.path, function(success)
             if not check_run(rid) then return end
             if not success then
                 _G.PokerQuest.echo("❌ 導航失誤，停止任務以免走錯路。")
                 _G.PokerQuest.stop(false)
                 return
             end
             _G.PokerQuest.echo("✅ 到達目的地")
             
             if step.handler then
                 _G.PokerQuest[step.handler](rid)
             elseif step.cmds then
                 for _, cmd in ipairs(step.cmds) do mud.send(cmd) end
                 if step.expect then
                     s.phase = "waiting_response"
                 else
                     _G.PokerQuest.advance_step(rid)
                 end
             else
                 _G.PokerQuest.advance_step(rid)
             end
        end)
        return
    end

    -- Handler 模式 (Explore)
    if step.handler then
        _G.PokerQuest[step.handler](rid)
        return
    end
end

function _G.PokerQuest.advance_step(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    local current = QUEST_STEPS[s.step_index]
    local next_name = current.next
    
    if next_name == "done" then
        _G.PokerQuest.echo("🎉 恭喜！任務流程結束。")
        _G.PokerQuest.stop(true)
        return
    end
    
    local next_idx = STEP_BY_NAME[next_name]
    if next_idx then
        s.step_index = next_idx
        MudUtils.safe_timer(1.0, _G.PokerQuest.process_step)
    else
        _G.PokerQuest.echo("⚠️ 未知下一步: " .. tostring(next_name))
        _G.PokerQuest.stop()
    end
end

-- ===== Handler 實作 =====


-- 階段一：探索並獵殺 Spade
function _G.PokerQuest.do_explore_spade(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    MudExplorer.config.target = "小黑桃(spade)"
    MudExplorer.config.max_laps = _G.PokerQuest.config.max_laps
    MudExplorer.config.disable_open_doors = true -- Disable door checking for this quest
    MudExplorer.config.debug = true -- Enable debug to investigate termination
    s.finding = true
    
    _G.PokerQuest.echo("🔍 開始搜索 Spade...")
    mud.send("wa") -- Ensure we are awake before exploring
    -- mud.send("l")
    
    -- 定義 MudExplorer 回調
    local function explore_cb(found, target_line)
        if not check_run(rid) then return end
        s.finding = false -- Immediately stop finding to prevent further hooks
        
        if found then
            _G.PokerQuest.echo("⚔️ 發現 Spade！準備戰鬥...")
            s.phase = "fighting"
            s.last_combat_time = os.time() -- Init combat timer
            s.corpse_offset = 0
            mud.send("wa") -- Wake up before fighting just in case
            
            -- Small delay to ensure commands clear (especially after door opening)
            MudUtils.safe_timer(0.5, function()
                if not check_run(rid) then return end
                mud.send(_G.PokerQuest.config.attack_cmd)
            end)
        else
            _G.PokerQuest.echo("❌ 搜索結束，未找到更多 Spade。")
            -- 如果沒找到石頭，可能需要在這裡重置? 或者直接結束?
            -- 這裡假設如果走完 max_laps 還沒找到，就任務失敗
            if not s.got_stone then
                 _G.PokerQuest.echo("⚠️ 搜索完畢仍未獲得石頭，任務失敗。")
                 _G.PokerQuest.stop()
            end
        end
    end
    _G.PokerQuest.explore_cb_ref = explore_cb

    -- 啟動探索 (或繼續)
    if s.got_stone then
         -- 理論上不該執行到這，因為有石頭就會跳下一步
         _G.PokerQuest.advance_step(rid)
    else
         MudExplorer.explore(explore_cb)
    end
end


-- 階段二：交付石頭
function _G.PokerQuest.do_deliver_stone(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    -- 需要先找出是第幾個 King (Diamond King)
    -- 這邊簡化邏輯：我們已經走到 King 的房間了 (由 path 保證)
    -- 我們可以 look 並解析，或者直接嘗試 give
    
    _G.PokerQuest.echo("🎁 尋找方塊國王給予石頭...")
    s.phase = "detecting_king"
    s.king_count = 0 -- Reset counter
    mud.send("l")
    -- Hook 會處理 detecting_king 的邏輯
end

-- ===== 戰鬥與物品處置 logic =====

function _G.PokerQuest.check_combat_clear(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if s.phase ~= "clearing" then return end

    -- Check time since last combat message
    -- Check combat state via MudCombat
    -- Also use local timer as double check if MudCombat is too strict/lenient?
    -- Actually, rely on MudCombat + local timer sync.
    if not MudCombat.is_fighting() then
        _G.PokerQuest.echo("✅ 戰鬥似乎已完全結束 (3秒無戰鬥訊息)")
        _G.PokerQuest.handle_loot_and_backtrack(rid)
    else
        -- Still fighting or just fought, check again in 1s
        MudUtils.safe_timer(1.0, _G.PokerQuest.check_combat_clear)
    end
end

function _G.PokerQuest.handle_loot_and_backtrack(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    _G.PokerQuest.echo("💰 戰利品階段，啟動 MudLoot...")
    MudLoot.process_loot({
        items = {"stone", "yellow stone"},
        loot_ground = true,
        sac = true,
        fallback_blind = true
    }, function()
        _G.PokerQuest.check_loot_result(rid)
    end)
end

function _G.PokerQuest.check_loot_result(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    -- Prevent double trigger (e.g. Instant + Timer)
    if s.phase == "backtracking" or s.phase == "navigating" or s.phase == "transitioning" then
        return
    end
    
    if s.got_stone then
        _G.PokerQuest.echo("💎 獲得黃色石頭！停止探索並回溯...")
        
        -- 取得回溯路徑
        local backtrack_path = MudExplorer.get_path_to_start()
        MudExplorer.stop()
        
        if backtrack_path and backtrack_path ~= "" then
            _G.PokerQuest.echo("🔙 回溯路徑: " .. backtrack_path)
            s.phase = "backtracking"
            -- 確保戰鬥完全結束後才走
            MudNav.walk(backtrack_path, function()
                 if not check_run(rid) then return end
                 -- 回到起點後，推進到下一步 (Delivery)
                 _G.PokerQuest.phase = "navigating" -- Ensure phase is correct
                 _G.PokerQuest.advance_step(rid)
            end)
        else
            _G.PokerQuest.echo("📍 已在起點或無路徑，直接推進...")
            _G.PokerQuest.advance_step(rid) 
        end
    else
        _G.PokerQuest.echo("❌ 未獲得石頭，恢復探索...")
        s.finding = true
        
        -- 定義 Resume Callback (與 start 一樣)
        local function explore_cb(found, target_line)
            if not check_run(rid) then return end
            s.finding = false -- Stop immediately to prevent accidental moves
            if found then
                _G.PokerQuest.echo("⚔️ 再次發現 Spade！戰鬥...")
                s.phase = "fighting"
                s.last_combat_time = os.time() -- Init combat timer
                s.corpse_offset = 0
                mud.send("wa")
                
                -- Start Combat Heatbeat
                MudUtils.safe_timer(0.5, function()
                    _G.PokerQuest.combat_heartbeat(rid)
                end)
            else
                if not s.got_stone then
                    _G.PokerQuest.echo("⚠️ 搜索完畢仍未獲得石頭，任務失敗。")
                    _G.PokerQuest.stop()
                end
            end
        end
        _G.PokerQuest.explore_cb_ref = explore_cb
        
        MudExplorer.resume(explore_cb)
    end
end


-- ===== Server Hook =====
if _G.PokerQuest.hook_installed and _G.PokerQuest._original_hook then
    _G.on_server_message = _G.PokerQuest._original_hook
end
if not _G.PokerQuest._original_hook then
    _G.PokerQuest._original_hook = _G.on_server_message
end
local base_hook = _G.PokerQuest._original_hook

_G.on_server_message = function(line, clean_line)
    pcall(function()
        if base_hook then base_hook(line, clean_line) end
        if _G.PokerQuest and _G.PokerQuest.on_server_message then
            _G.PokerQuest.on_server_message(line, clean_line)
        end
    end)
end
_G.PokerQuest.hook_installed = true

function _G.PokerQuest.on_server_message(line, clean_line)
    if not _G.PokerQuest.state.running then return end
    local s = _G.PokerQuest.state
    
    -- 委派給 Modules
    if s.finding then MudExplorer.on_server_message(clean_line) end
    MudNav.on_server_message(line, clean_line)
    
    -- 監測戰鬥訊息 (Delegated to MudCombat)
    if MudCombat.on_server_message(clean_line) then
        s.last_combat_time = os.time()
        -- Ensure we are in fighting state if heavy combat detected
        -- But IGNORE if it's a "Victory" message (defeat of mob)
        -- Because MudCombat interprets "Spirit flying to heaven" as combat too?
        -- Actually, "魂歸西天" is NOT in MudCombat keywords.
        -- But "口吐鮮血" (final blow) IS.
        -- If we just killed them, we shouldn't switch BACK to fighting immediately.
        
        local is_killing_blow = string.find(clean_line, "魂歸西天") or string.find(clean_line, "得向閻羅王報到")
        
        if not is_killing_blow and (s.phase == "finding" or s.phase == "looting" or s.phase == "clearing") then
             _G.PokerQuest.echo("⚠️ 偵測到戰鬥活動，切換回戰鬥狀態！")
             s.phase = "fighting"
             _G.PokerQuest.combat_heartbeat(MudUtils.run_id)
        end
    end
    
    -- Specific Fix: "You are already fighting!" recovery
    if string.find(clean_line, "身陷戰鬥中") then
        _G.PokerQuest.echo("⚠️ 系統提示身陷戰鬥，強制重置戰鬥計時！")
        MudCombat.active()
        s.phase = "fighting"
        s.last_combat_time = os.time()
        _G.PokerQuest.combat_heartbeat(MudUtils.run_id)
    end
    
    -- Specific Fix: Instant Loot Detection (Got Stone)
    if string.find(clean_line, "從.*屍體.*拿出.*黃色石頭") or string.find(clean_line, "獲得.*黃色石頭") then
        _G.PokerQuest.echo("💎 [Instant] 偵測到獲得黄石！")
        s.got_stone = true
        _G.PokerQuest.check_loot_result(MudUtils.run_id) 
    end

    -- Proactive Ground Loot (During exploration)
    if s.phase == "finding" or s.finding then
        if string.find(clean_line, "看起來很像黃金的黃色石頭") or string.find(clean_line, "yellow stone") then
            _G.PokerQuest.echo("✨ [Found] 發現地面上有石子，嘗試拾取...")
            mud.send("get stone")
            -- We don't stop exploration yet, but MudLoot/Inventory check will catch it if success messenger arrives
        end
    end
    
    -- Route messages to MudLoot
    if MudLoot and MudLoot.on_server_message then
        MudLoot.on_server_message(line, clean_line)
    end

    -- 戰鬥處理
    if s.phase == "fighting" then
        if string.find(clean_line, "魂歸西天了") then
             if string.find(clean_line, "Spade") then
                 s.kills = s.kills + 1
                 _G.PokerQuest.echo("💀 擊殺 Spade (Total: " .. s.kills .. ")")
                 -- 擊殺重點目標，進入 clearing 檢查
                 s.phase = "clearing"
                 s.last_combat_time = os.time() -- Reset timer
                 MudUtils.safe_timer(1.0, _G.PokerQuest.check_combat_clear)
             else
                 s.corpse_offset = s.corpse_offset + 1
             end
        end
        
        -- Low MV/MP Handling
        if string.find(clean_line, "你的移動力不足") or string.find(clean_line, "你的法力不足") then
             _G.PokerQuest.echo("⚠️ 精力不足，嘗試恢復...")
             mud.send("c ref")
             MudUtils.safe_timer(1.0, function()
                 if check_run(s.run_id) then mud.send(_G.PokerQuest.config.attack_cmd) end
             end)
        end
        
        if string.find(clean_line, "Spade .*離開了") then
             _G.PokerQuest.echo("⚠️ 目標逃跑了！恢復探索...")
             s.phase = "finding"
             s.finding = true -- IMPORTANT: Enable MudExplorer hooks
             if _G.PokerQuest.explore_cb_ref then
                 MudExplorer.resume(_G.PokerQuest.explore_cb_ref)
             end
        end

        -- 追加攻擊?
        if string.find(clean_line, "你的體力逐漸地恢復") then
             -- 戰鬥中恢復，可能沒在打? 確保攻擊
             mud.send(_G.PokerQuest.config.attack_cmd)
        end
    end
    
    -- 撿石頭確認
    if s.phase == "looting" then
        if string.find(clean_line, "黃色石頭") and string.find(clean_line, "你從") then
            s.got_stone = true
        end
    end
    
    -- Deliver Stone: Detecting King
    if s.phase == "detecting_king" then
        -- Count occurrences of "king" or "King" (case insensitive check might be safer, but log showed "king" in parens)
        -- Log: 方塊國王(king) ,正坐在這裡 .
        if string.find(clean_line, "(king)", 1, true) or string.find(clean_line, "King", 1, true) then
            s.king_count = (s.king_count or 0) + 1
            
            if string.find(clean_line, "方塊") then
                 local target_cmd = (s.king_count == 1) and "king" or (s.king_count .. ".king")
                 _G.PokerQuest.echo("🎁 找到方塊國王 (Target: " .. target_cmd .. ")，嘗試給予石頭...")
                 s.phase = "delivering"
                 mud.send("gi stone " .. target_cmd)
            end
        end
    end

    -- Deliver Stone: Success Check
    if s.phase == "delivering" then
        if string.find(clean_line, "你把.*給了.*King") or string.find(clean_line, "方塊國王說道") then
            _G.PokerQuest.echo("✨ 石頭交付成功！")
            s.phase = "transitioning" -- Lock phase immediately to prevent double trigger
            _G.PokerQuest.advance_step(s.run_id)
        end
    end
    
    -- Step Expectation Check
    local step = QUEST_STEPS[s.step_index]
    if s.phase == "waiting_response" and step and step.expect then
        if string.find(clean_line, step.expect, 1, true) then
            _G.PokerQuest.echo("✨ 觸發劇情: " .. step.expect)
            _G.PokerQuest.advance_step(s.run_id)
        end
    end
end

MudUtils.show_script_usage("PokerQuest", {
    "PokerQuest.start()   - 🚀 啟動任務",
    "PokerQuest.stop()    - 🛑 停止任務",
    "PokerQuest.status()  - 📊 查看狀態"
})

-- Persistent Combat Heartbeat
function _G.PokerQuest.combat_heartbeat(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    -- Only run if valid fighting phase
    if s.phase ~= "fighting" then return end
    
    -- Use config attack command
    if _G.PokerQuest.config.attack_cmd then
        -- Debug message to confirm heartbeat is alive
        -- _G.PokerQuest.echo("⚔️ [Heartbeat] Attack!") 
        mud.send(_G.PokerQuest.config.attack_cmd)
    end
    
    -- Schedule next beat (e.g. every 2 seconds)
    MudUtils.safe_timer(2.0, function() 
        _G.PokerQuest.combat_heartbeat(rid) 
    end)
end

return _G.PokerQuest
