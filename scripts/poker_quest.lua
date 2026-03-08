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

-- ===== 設定 =====
_G.PokerQuest.config = {
    entry_path = {
        "s", "s", "s", "s", "s", "s", "e", "e", "u", "u", "u",
        {cmd="u", id="20ce628eff898093aae8aea12ce15043ad2c599a254804579c1956afff2b4bef"},
    },
    deliver_path = {
        "n", "n", "w", "w", "n", "n",
        {cmd="w", id="6faf86d9c11f591577f24cab47c7a2f29d980e9f424d85dcb97af994559b3f15"},
    },
    attack_cmd = "ear spade",
    debug = false,
    max_laps = 5,
}

-- ===== 任務步驟 =====
local QUEST_STEPS = {
    {name="explore_spade",  handler="do_explore_spade", expect="success_or_fail", next="deliver_stone"},
    {name="deliver_stone",  path=_G.PokerQuest.config.deliver_path, handler="do_deliver_stone", expect="把 黃色石頭 給了", next="talk_queen"},
    {name="talk_queen",     path={
        "e", "n", "n", "e", "e",
        {cmd="n", id="68e309f3e2252bd02102e43fcc000c90a3551d36791d21974f54ebf92e929c21"},
    }, cmds={"say goodmorning"}, expect="黑桃王后說道: 我可以告訴你是 'ireallywantleave'", next="go_palace"},
    -- Use Heart Queen response or generic say confirm
    {name="go_palace",      path={
        "s", "s", "s", "u", "u",
        {cmd="u", id="fd4a9da729fd717f3b7595f056e6a313877e48a865d17aaa4c73f69cbef078c6"},
    }, cmds={"say ireallywantleave"}, expect="紅心女王說道: 好吧 !我再試一試這咒語", next="done"},
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
    _G.PokerQuest.state.recovering = false
    
    MudUtils.start_log("poker")
    MudUtils.register_quest("PokerQuest", _G.PokerQuest.stop)
    
    _G.PokerQuest.echo("🚀 啟動撲克王國任務！")
    mud.emit("quest_started", {name = "PokerQuest"})

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
    mud.emit("quest_stopped", {name = "PokerQuest", success = is_success or false, kills = s.kills})
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
                     -- Script Wait Timeout
                     MudUtils.safe_timer(5.0, function()
                         if check_run(s.run_id) and s.phase == "waiting_response" then
                             _G.PokerQuest.echo("❌ 等待回應超時 (" .. step.expect .. ")！任務失敗。")
                             _G.PokerQuest.stop()
                         end
                     end)
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

-- Persistent Combat Heartbeat (定期自動攻擊)
function _G.PokerQuest.combat_heartbeat(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if s.phase ~= "fighting" then return end

    if not s.recovering and _G.PokerQuest.config.attack_cmd then
        if s.non_target_combat then
            -- 被非目標怪物攻擊，嘗試逃跑
            mud.send("flee")
        else
            mud.send(_G.PokerQuest.config.attack_cmd)
        end
    end

    MudUtils.safe_timer(2.5, function()
        _G.PokerQuest.combat_heartbeat(rid)
    end)
end

-- 共用的 explore callback 工廠函數
local function make_explore_cb(rid)
    return function(found, target_line)
        if not check_run(rid) then return end
        local s = _G.PokerQuest.state
        s.finding = false
        
        if found then
            _G.PokerQuest.echo("⚔️ 發現 Spade！準備戰鬥...")
            s.phase = "fighting"
            s.non_target_combat = false
            s.last_combat_time = os.time()
            s.corpse_offset = 0
            mud.send("wa")
            
            MudUtils.safe_timer(0.5, function()
                if not check_run(rid) then return end
                mud.send(_G.PokerQuest.config.attack_cmd)
                _G.PokerQuest.combat_heartbeat(rid)
            end)
        else
            _G.PokerQuest.echo("❌ 搜索結束，未找到更多 Spade。")
            if not s.got_stone then
                _G.PokerQuest.echo("⚠️ 搜索完畢仍未獲得石頭，任務失敗。")
                _G.PokerQuest.stop()
            end
        end
    end
end

-- 階段一：探索並獵殺 Spade
function _G.PokerQuest.do_explore_spade(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    MudExplorer.config.target = "小黑桃(spade)"
    MudExplorer.config.max_laps = _G.PokerQuest.config.max_laps
    MudExplorer.config.disable_open_doors = true
    MudExplorer.config.debug = true
    s.finding = true
    
    _G.PokerQuest.echo("🔍 開始搜索 Spade...")
    mud.send("wa")
    
    local explore_cb = make_explore_cb(rid)
    _G.PokerQuest.explore_cb_ref = explore_cb

    if s.got_stone then
        _G.PokerQuest.advance_step(rid)
    else
        MudExplorer.explore(explore_cb)
    end
end


-- 階段二：交付石頭
function _G.PokerQuest.do_deliver_stone(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    _G.PokerQuest.echo("🎁 尋找方塊國王給予石頭...")
    s.phase = "detecting_king"
    s.king_count = 0 -- Reset counter
    mud.send("l")
    
    -- Timeout protection
    MudUtils.safe_timer(5.0, function()
        if check_run(s.run_id) and s.phase == "detecting_king" then
            _G.PokerQuest.echo("❌ 尋找方塊國王超時 (5秒內未發現)！任務失敗。")
            _G.PokerQuest.stop()
        end
    end)
end

-- ===== 戰鬥與物品處置 logic =====

function _G.PokerQuest.check_combat_clear(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if s.phase ~= "clearing" then return end

    s.clear_checks = (s.clear_checks or 0) + 1

    if not MudCombat.is_fighting() or s.clear_checks >= 10 then
        if s.clear_checks >= 10 then
            _G.PokerQuest.echo("✅ 戰鬥清除超時，強制進入搜刮")
        else
            _G.PokerQuest.echo("✅ 戰鬥已結束 (" .. s.clear_checks .. "次檢查)")
        end
        s.clear_checks = 0
        _G.PokerQuest.handle_loot_and_backtrack(rid)
    else
        MudUtils.safe_timer(1.0, _G.PokerQuest.check_combat_clear)
    end
end

function _G.PokerQuest.handle_loot_and_backtrack(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    
    _G.PokerQuest.echo("💰 戰利品階段，啟動 MudLoot...")
    MudLoot.process_loot({
        items = {"stone"},
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
        
        local explore_cb = make_explore_cb(rid)
        _G.PokerQuest.explore_cb_ref = explore_cb
        MudExplorer.resume(explore_cb)
    end
end


-- ===== Server Hook =====
-- 使用 Hook Registry（不再手動包裹 _G.on_server_message）
MudUtils.register_hook("PokerQuest", function(line, clean_line)
    if _G.PokerQuest and _G.PokerQuest.on_server_message then
        _G.PokerQuest.on_server_message(line, clean_line)
    end
end)

function _G.PokerQuest.on_server_message(line, clean_line)
    if not _G.PokerQuest.state.running then return end
    local s = _G.PokerQuest.state

    -- 睡覺偵測：凍原山頂會自動入睡，需要 wake 後才能行動
    if string.find(clean_line, "你正在睡覺") or string.find(clean_line, "睡得很熟") then
        MudUtils.safe_timer(1.5, function()
            mud.send("wa")
        end)
        return
    end

    -- MudNav/MudExplorer 已透過 Hook Registry 自行接收訊息

    -- 監測戰鬥訊息 (Delegated to MudCombat)
    if MudCombat.on_server_message(clean_line) then
        s.last_combat_time = os.time()
        -- Ensure we are in fighting state if heavy combat detected
        -- But IGNORE if it's a "Victory" message (defeat of mob)
        -- Because MudCombat interprets "Spirit flying to heaven" as combat too?
        -- Actually, "魂歸西天" is NOT in MudCombat keywords.
        -- But "口吐鮮血" (final blow) IS.
        -- If we just killed them, we shouldn't switch BACK to fighting immediately.
        local is_killing_blow = string.find(clean_line, "魂歸西天") 
            or string.find(clean_line, "得向閻羅王報到")
            or string.find(clean_line, "倒地")
            or string.find(clean_line, "慘叫")
            or string.find(clean_line, "氣絕")
            or string.find(clean_line, "死亡")

        local is_false_positive = string.find(clean_line, "你想攻擊的對象不在這裡")
            or string.find(clean_line, "這裡禁止攻擊")

        if not is_killing_blow and not is_false_positive and (s.phase == "finding" or s.phase == "looting" or s.phase == "clearing") then
             -- 判斷是否為非目標怪物（非 Spade）
             s.non_target_combat = not string.find(clean_line, "[Ss]pade")
             if s.non_target_combat then
                 _G.PokerQuest.echo("⚠️ 被非目標怪物攻擊，嘗試逃跑！")
             else
                 _G.PokerQuest.echo("⚠️ 偵測到戰鬥活動，切換回戰鬥狀態！")
             end
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

    -- Flee 成功偵測：逃離非目標戰鬥後恢復探索
    if s.non_target_combat and string.find(clean_line, "你逃離了戰鬥") then
        _G.PokerQuest.echo("🏃 成功逃離非目標戰鬥，恢復探索...")
        s.non_target_combat = false
        s.phase = "finding"
        MudExplorer.explore(make_explore_cb(s.run_id))
    end

    -- Specific Fix: Instant Loot Detection (Got Stone)
    if not s.got_stone and (string.find(clean_line, "從.*屍體.*拿出.*黃色石頭") or string.find(clean_line, "獲得.*黃色石頭")) then
        _G.PokerQuest.echo("💎 [Instant] 偵測到獲得黄石！")
        s.got_stone = true
        mud.emit("poker_stone_found")
    end

    -- Proactive Ground Loot (During exploration)
    if s.phase == "finding" or s.finding then
        if string.find(clean_line, "看起來很像黃金的黃色石頭") or string.find(clean_line, "yellow stone") then
            _G.PokerQuest.echo("✨ [Found] 發現地面上有石子，嘗試拾取...")
            mud.send("get stone")
            -- We don't stop exploration yet, but MudLoot/Inventory check will catch it if success messenger arrives
        end
    end
    
    -- MudLoot 亦透過 Hook Registry 自行接收訊息

    -- 戰鬥處理
    if s.phase == "fighting" then
        if string.find(clean_line, "魂歸西天了") then
             if string.find(clean_line, "Spade") then
                 s.kills = s.kills + 1
                 mud.emit("poker_spade_killed", {total = s.kills})
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
             if not s.recovering then
                 _G.PokerQuest.echo("⚠️ 精力不足，暫停攻擊並嘗試恢復...")
                 s.recovering = true
                 mud.send("c ref")
                 -- 安全解鎖防呆，避免 c ref 被打斷永遠卡在 recovering
                 MudUtils.safe_timer(5.0, function()
                     if check_run(s.run_id) and s.recovering then
                         s.recovering = false
                         _G.PokerQuest.combat_heartbeat(s.run_id)
                     end
                 end)
             end
        end
        
        if string.find(clean_line, "Spade .*離開了") then
             _G.PokerQuest.echo("⚠️ 目標逃跑了！恢復探索...")
             s.recovering = false -- 重置
             s.phase = "finding"
             s.finding = true -- IMPORTANT: Enable MudExplorer hooks
             if _G.PokerQuest.explore_cb_ref then
                 MudExplorer.resume(_G.PokerQuest.explore_cb_ref)
             end
        end

        -- 追加攻擊與恢復確認
        if string.find(clean_line, "你的體力逐漸地恢復") then
             if s.recovering then
                 _G.PokerQuest.echo("✨ 精力恢復，繼續攻擊！")
                 s.recovering = false
                 if s.phase == "fighting" then
                     _G.PokerQuest.combat_heartbeat(s.run_id)
                 end
             else
                 -- 戰鬥中恢復，可能沒在打? 確保攻擊
                 if s.phase == "fighting" then
                     mud.send(_G.PokerQuest.config.attack_cmd)
                 end
             end
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
        -- Count occurrences of "king" or "King" 
        -- Log: 方塊國王(King) ,正坐在這裡 .
        if string.find(clean_line, "國王%([kK]ing%)") then
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

-- combat_heartbeat 已移至 Handler 區塊（L241）

return _G.PokerQuest
