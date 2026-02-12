-- ============================================================
-- ItemFarm v2.1 - 自動打怪收集物品 (多任務輪替)
-- ============================================================
-- 模式：
--   summon  = 前往安全點 → 召喚 → 攻擊
--   direct  = 前往怪物處 → (dispel → buff) → 攻擊
-- 流程：
-- 1. 查詢當前任務的怪物是否重生
-- 2. 未重生 → 跳到下一個任務；全部都沒重生 → 等待後重新輪替
-- 3. 重生 → 依模式擊殺 → 收集戰利品 → 儲存 → 休息 → 重複
-- ============================================================

_G.ItemFarm = _G.ItemFarm or {}

-- ===== Local Cache (效能優化) =====
-- local mud = mud -- 避免快取宿主物件，使用全域查找以保證穩定性
local string = string
local table = table
local os = os
local tonumber = tonumber
local ipairs = ipairs
local pairs = pairs
local math = math

-- ===== 全域設定 =====
_G.ItemFarm.config = {
    mp_threshold = 50,       -- MP 百分比閾值
    hp_threshold = 50,        -- 全局 HP 門檻 (0 表示預設不檢查)
    poll_interval = 30,      -- 全部未重生時的等待秒數
    rest_cmd = "sleep",          -- 休息指令
    score_interval = 20,     -- score 指令最小間隔 (秒)
    require_sanctuary = false, -- 是否強制要求聖光
}

-- ===== 任務列表 =====
-- search_type: "quest" (偵測「他正在這個世界中」) / "locate" (偵測「攜帶著」)
-- mode: "summon" (召喚後攻擊) / "direct" (直接到場攻擊)
-- dispel_cmd: 攻擊前需重試直到成功的指令（如 dispel magic）
-- buff_cmds: dispel 成功後執行的 buff 指令
_G.ItemFarm.jobs = {
    {
        name = "商務間諜",
        mode = "summon",             -- summon 或 direct
        search_type = "quest",
        search_cmd = "q 2.spy",
        target_mob = "商務間諜",
        summon_cmd = "c sum spy",
        attack_cmd = "c fl spy",
        path_to_mob = "w;s;2e",
        path_to_storage = "w;w;n;e",
        loot_items = {"anesthetic", "grating"},
        remove_nodrop = {"anesthetic", "grating"},
        sac_corpse = true,
    },
    {
        name = "街頭混混",
        mode = "summon",
        search_type = "quest",
        search_cmd = "q 28.boy",
        target_mob = "街頭混混",
        summon_cmd = "c sum boy",
        attack_cmd = "c fl boy",
        path_to_mob = "w",
        path_to_storage = "e",
        loot_items = {"take"},
        remove_nodrop = {},
        sac_corpse = true,
    },
    {
        name = "不動明王",
        mode = "direct",
        search_type = "quest",
        search_cmd = "q 6.sentinel",
        target_mob = "不動明王",
        attack_cmd = "c star;c star;c star",
        dispel_cmd = "c 'dispel m' sentinel",
        dispel_indicator = "白色聖光",    -- look 後此字消失 = dispel 成功
        hp_threshold = 100,               -- 特定怪物才檢查血量
        hp_recover_cmd = "c heal",         -- 自定義恢復 HP 的指令
        buff_cmds = {"c sa", "c pro", "c b"},
        path_to_mob = "recall;3w;4s;ta wizard help;7w;7n;6u;7n",
        path_to_storage = "recall;3n;e",
        loot_items = {"sword", "potato", "hamburg"},
        remove_nodrop = {},
        sac_corpse = true,
        require_sanctuary = true,
    },
}

-- ===== 狀態 =====
_G.ItemFarm.state = {
    running = false,
    run_id = 0,            -- 防止 Timer 競爭條件
    stage = "idle",
    current_mp = 0,
    max_mp = 0,
    current_hp = 0,
    max_hp = 0,
    found_target = false,
    loot_count = 0,
    search_count = 0,
    summon_retries = 0,
    dispel_retries = 0,
    current_job = 1,       -- 當前任務索引
    jobs_checked = 0,      -- 本輪已檢查的任務數
    last_score_time = 0,   -- 上次發送 score 的時間
    has_sanctuary = false, -- 是否有聖光
    -- 路徑佇列（prompt 驅動）
    path_queue = {},
    path_index = 0,
    path_callback = nil,
    path_paused = false,
    walking = false,       -- 是否正在行走中
    path_paused = false,
    walking = false,       -- 是否正在行走中
}

-- ===== Timer Helper (防止舊 Timer 觸發) =====
function _G.ItemFarm.safe_timer(seconds, callback_code)
    -- 自動將 run_id 注入到 callback 中
    -- 假設 callback 是 "_G.ItemFarm.foo()" 或 "_G.ItemFarm.foo(arg)"
    -- 我們將其改寫為 "_G.ItemFarm.foo(..., <run_id>)"
    -- 但這涉及字串解析太複雜。
    -- 簡單策略：要求所有 callback 手動檢查 state.run_id
    -- 或者，我們在這裡封裝一個匿名函數？不行，mud.timer 只收字串。
    
    -- 改用約定：所有 Timer 觸發的函數，最後一個參數必須是 run_id
    -- 呼叫時： mud.timer(sec, "_G.ItemFarm.func(" .. current_run_id .. ")")
    mud.timer(seconds, callback_code)
end

-- 檢查 run_id 是否有效
local function check_run(run_id)
    if not run_id then return true end -- 相容舊呼叫 (過渡期)
    return run_id == _G.ItemFarm.state.run_id
end

-- ===== 輔助函數 =====
function _G.ItemFarm.job()
    return _G.ItemFarm.jobs[_G.ItemFarm.state.current_job]
end

-- 解析指令字串，展開重複語法 (7w → 7 次 w)
local function parse_cmds(str)
    local result = {}
    for cmd in string.gmatch(str, "[^;]+") do
        cmd = cmd:match("^%s*(.-)%s*$")  -- trim
        if cmd ~= "" then
            local count, actual = cmd:match("^(%d+)(%a.*)$")
            if count then
                for _ = 1, tonumber(count) do
                    table.insert(result, actual)
                end
            else
                table.insert(result, cmd)
            end
        end
    end
    return result
end

-- 即時發送指令（用於攻擊、buff 等不需要體力檢測的場景）
local function send_cmds(str)
    for _, cmd in ipairs(parse_cmds(str)) do
        mud.send(cmd)
    end
end


-- ===== Prompt 驅動路徑行走 =====
-- 送一個指令 → 等 MUD prompt → 再送下一個
function _G.ItemFarm.walk_path(str, callback)
    local s = _G.ItemFarm.state
    s.path_queue = parse_cmds(str)
    s.path_index = 1
    s.path_callback = callback
    s.path_paused = false
    s.walking = true
    _G.ItemFarm.walk_send(s.run_id)
end

-- 發送當前指令（不前進 index，等 prompt 來再前進）
function _G.ItemFarm.walk_send(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    
    if s.path_index > #s.path_queue then
        -- 所有指令完成
        s.walking = false
        s.path_queue = {}
        s.path_index = 0
    if s.path_callback then
        mud.timer(0.5, s.path_callback) -- 這裡 callback 已經包含了 run_id (如果構建正確)
        -- 注意：callback 字串本身需要包含 run_id
    end
    return
end

local cmd = s.path_queue[s.path_index]
mud.send(cmd)
-- 等待 hook 偵測 prompt 後自動前進
end

-- Prompt 到達後前進到下一個指令（由 hook 呼叫）
function _G.ItemFarm.walk_advance()
    -- 這裡由 Hook 觸發，無法傳遞 run_id，但可以用 current state check
    -- 嚴格來說，Hook 觸發的是「當前」週期，所以 implicitly 是 current run_id
    local s = _G.ItemFarm.state
    s.path_index = s.path_index + 1
    _G.ItemFarm.walk_send(s.run_id)
end

function _G.ItemFarm.recover_stamina(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    mud.echo("⚡ 施放 refresh 恢復體力...")
    mud.send("c ref")
    -- 等待 hook 偵測「你的體力逐漸地恢復」才繼續
end

-- 體力恢復後，解鎖並重試當前指令
function _G.ItemFarm.walk_resume()
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    s.path_paused = false
    _G.ItemFarm.walk_send(s.run_id)
end

-- ===== 初始化 =====
function _G.ItemFarm.init()
    mud.echo("═══════════════════════════════════════════")
    mud.echo("  🎯 ItemFarm v2.1 - 自動打怪收集")
    mud.echo("═══════════════════════════════════════════")
    mud.echo("  指令:")
    mud.echo("    ItemFarm.start()  - 開始自動收集")
    mud.echo("    ItemFarm.stop()   - 停止")
    mud.echo("    ItemFarm.status() - 顯示狀態")
    mud.echo("  任務數: " .. #_G.ItemFarm.jobs)
    for i, j in ipairs(_G.ItemFarm.jobs) do
        local m = j.mode or "summon"
        mud.echo("    [" .. i .. "] " .. j.name .. " (" .. m .. ")")
    end
    mud.echo("═══════════════════════════════════════════")
end

-- ===== 主要函數 =====
function _G.ItemFarm.start()
    if _G.ItemFarm.state.running then
        mud.echo("⚠️ 已經在運行中")
        return
    end
    
    local s = _G.ItemFarm.state
    s.running = true
    s.run_id = (s.run_id or 0) + 1  -- 新的執行週期
    s.stage = "idle"
    s.loot_count = 0
    s.summon_retries = 0
    s.current_job = 1
    s.jobs_checked = 0
    s.has_sanctuary = false -- 重置聖光狀態
    
    local j = _G.ItemFarm.job()
    mud.echo("🎯 開始自動收集 (" .. #_G.ItemFarm.jobs .. " 個任務)")
    mud.echo("   當前任務: [" .. s.current_job .. "] " .. j.name)
    _G.ItemFarm.search(s.run_id)
end

function _G.ItemFarm.stop()
    _G.ItemFarm.state.running = false
    _G.ItemFarm.state.stage = "idle"
    mud.echo("🛑 已停止自動收集")
    mud.echo("   本次收集: " .. _G.ItemFarm.state.loot_count .. " 次")
end

function _G.ItemFarm.status()
    local s = _G.ItemFarm.state
    mud.echo("📊 ItemFarm 狀態:")
    mud.echo("   運行中: " .. (s.running and "是" or "否"))
    mud.echo("   階段: " .. s.stage)
    mud.echo("   收集次數: " .. s.loot_count)
    if s.running then
        local j = _G.ItemFarm.job()
        mud.echo("   當前任務: [" .. s.current_job .. "] " .. j.name)
    end
    mud.echo("   任務列表:")
    for i, j in ipairs(_G.ItemFarm.jobs) do
        local marker = (i == s.current_job and s.running) and " ◀" or ""
        local disabled = j.disabled and " [已停用]" or ""
        mud.echo("     [" .. i .. "] " .. j.name .. disabled .. marker)
    end
end

-- ===== 任務輪替 =====
function _G.ItemFarm.next_job()
    local s = _G.ItemFarm.state
    s.jobs_checked = s.jobs_checked + 1
    
    -- 所有任務都檢查過了（或被停用）
    -- 檢查是否還有可用任務
    local active_count = 0
    for _, j in ipairs(_G.ItemFarm.jobs) do
        if not j.disabled then active_count = active_count + 1 end
    end
    
    if active_count == 0 then
        mud.echo("⚠️ 所有任務已停用，停止運行")
        _G.ItemFarm.stop()
        return
    end
    
    if s.jobs_checked >= active_count then
        s.jobs_checked = 0
        s.stage = "waiting"
        mud.echo("⏳ 所有目標皆未重生，" .. _G.ItemFarm.config.poll_interval .. " 秒後重新輪替...")
        mud.send(_G.ItemFarm.config.rest_cmd)
        mud.timer(_G.ItemFarm.config.poll_interval, "_G.ItemFarm.search(" .. s.run_id .. ")")
        return
    end
    
    -- 跳到下一個未停用的任務
    local total = #_G.ItemFarm.jobs
    for _ = 1, total do
        s.current_job = (s.current_job % total) + 1
        local j = _G.ItemFarm.job()
        if not j.disabled then
            mud.echo("🔄 切換任務: [" .. s.current_job .. "] " .. j.name)
            s.stage = "idle"
            mud.timer(1.0, "_G.ItemFarm.search(" .. s.run_id .. ")")
            return
        end
    end
    -- 所有任務都停用
    mud.echo("⚠️ 所有任務已停用")
    _G.ItemFarm.stop()
end

-- ===== 階段函數 =====

-- 1. 搜尋怪物
function _G.ItemFarm.search()
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "idle" and 
       _G.ItemFarm.state.stage ~= "waiting" and 
       _G.ItemFarm.state.stage ~= "resting" then
        return
    end
    
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    
    -- 跳過已停用的任務
    if j.disabled then
        _G.ItemFarm.next_job()
        return
    end
    
    s.stage = "searching"
    s.found_target = false
    
    mud.echo("🔍 [" .. j.name .. "] 查詢目標...")
    if j.search_type ~= "quest" then
        mud.send("wa")
    end
    mud.send(j.search_cmd)
    
    -- 超時：3 秒後未偵測到 → 視為未重生
    mud.timer(3.0, "_G.ItemFarm.search_timeout(" .. s.run_id .. ")")
end

function _G.ItemFarm.search_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "searching" then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.search_count = _G.ItemFarm.state.search_count + 1
    mud.echo("❌ [" .. j.name .. "] 目標未重生")
    
    -- 跳到下一個任務
    _G.ItemFarm.next_job()
end

-- 2. 前往目標
function _G.ItemFarm.go_and_fight()
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local mode = j.mode or "summon"
    _G.ItemFarm.state.stage = "traveling"
    _G.ItemFarm.state.jobs_checked = 0  -- 重置輪替計數
    mud.echo("🚶 [" .. j.name .. "] 前往目標位置...")
    mud.send("wa")
    
    local callback
    if mode == "direct" then
        callback = "_G.ItemFarm.engage_direct(" .. _G.ItemFarm.state.run_id .. ")"
    else
        -- 召喚前先檢查狀態
        callback = "_G.ItemFarm.check_status_before_summon(" .. _G.ItemFarm.state.run_id .. ")"
    end
    _G.ItemFarm.walk_path(j.path_to_mob, callback)
end

-- 2a. 召喚前檢查狀態
function _G.ItemFarm.check_status_before_summon(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local s = _G.ItemFarm.state
    s.stage = "checking_status_pre_summon"
    s.last_score_time = os.time()
    mud.echo("📊 召喚前檢查狀態 (發送 score)...")
    mud.send("score")
    mud.send("save")
end

-- 評估召喚前狀態
function _G.ItemFarm.evaluate_status_before_summon()
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config

    local j_hp_threshold = j.hp_threshold or cfg.hp_threshold
    local hp_ok = (s.max_hp == 0) or (j_hp_threshold == 0) or ((s.current_hp / s.max_hp) * 100 >= j_hp_threshold)
    local mp_ok = (s.max_mp == 0) or ((s.current_mp / s.max_mp) * 100 >= cfg.mp_threshold)
    local req_sanctuary = cfg.require_sanctuary
    if j.require_sanctuary ~= nil then req_sanctuary = j.require_sanctuary end
    local sanc_ok = not req_sanctuary or s.has_sanctuary

    if not hp_ok or not mp_ok then
        local reason = not hp_ok and "HP" or "MP"
        local threshold = not hp_ok and j_hp_threshold or cfg.mp_threshold
        mud.echo("⚠️ " .. reason .. " 不足 (" .. threshold .. "% 門檻)，先休息回滿...")
        _G.ItemFarm.rest_and_repeat() -- 暫時用原地休息，或可透過 after_return 邏輯
        -- 若需要回儲存點：
        -- s.stage = "returning"
        -- _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return()")
        return
    end

    if not sanc_ok then
        mud.echo("🛡️ 聖光不足，嘗試施放 'c san'...")
        mud.send("c san")
        mud.timer(2.0, "_G.ItemFarm.check_status_before_summon(" .. s.run_id .. ")")
        return
    end

    mud.echo("✅ 狀態良好，開始召喚！")
    _G.ItemFarm.summon_and_attack()
end

-- 2b. 直接交戰模式（到場 → 驗證 mob → dispel → buff → 攻擊）
function _G.ItemFarm.engage_direct(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "traveling" then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    -- 先 look 確認 mob 是否在場
    s.stage = "verifying_mob"
    mud.echo("🔍 [​" .. j.name .. "] 確認目標是否在場...")
    mud.send("l")
    -- 超時 3 秒 → mob 不在
    mud.timer(3.0, "_G.ItemFarm.verify_mob_timeout(" .. s.run_id .. ")")
end


-- mob 不在場 → 用 search_cmd 確認是死亡還是迷路
function _G.ItemFarm.verify_mob_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "verifying_mob" then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    mud.echo("❓ [​" .. j.name .. "] 目標不在場，查詢狀態...")
    s.stage = "verifying_loc"
    mud.send(j.search_cmd)
    -- 超時 3 秒 → mob 已死
    mud.timer(3.0, "_G.ItemFarm.verify_loc_timeout(" .. s.run_id .. ")")
end

-- search_cmd 超時 → mob 已死，返回休息
function _G.ItemFarm.verify_loc_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "verifying_loc" then return end
    
    local j = _G.ItemFarm.job()
    mud.echo("💠 [​" .. j.name .. "] 目標已死亡，返回休息等待重生...")
    _G.ItemFarm.state.stage = "returning"
    _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return(" .. s.run_id .. ")")
end

-- mob 驗證通過後，開始 dispel 或直接攻擊
function _G.ItemFarm.start_dispel_or_attack()
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    if j.dispel_cmd and j.dispel_indicator then
        -- 需要 dispel：發送 dispel + look 來檢查
        s.stage = "dispelling"
        s.dispel_retries = 0
        s.dispel_retries = 0
        mud.echo("🔮 [" .. j.name .. "] Dispel 中...")
        mud.send(j.dispel_cmd)
        mud.timer(1.5, '_G.ItemFarm.check_dispel(' .. s.run_id .. ')')
    elseif j.dispel_cmd then
        -- 有 dispel_cmd 但沒 indicator，用舊邏輯
        s.stage = "dispelling"
        s.dispel_retries = 0
        mud.echo("🔮 [" .. j.name .. "] Dispel 中...")
        mud.send(j.dispel_cmd)
    else
        -- 不需要 dispel
        _G.ItemFarm.buff_and_attack(s.run_id)
    end
end

-- Dispel 後發送 look 檢查 indicator
function _G.ItemFarm.check_dispel(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "dispelling" then return end
    
    _G.ItemFarm.state.stage = "checking_dispel"
    mud.send("l")
    mud.timer(3.0, '_G.ItemFarm.check_dispel_timeout(' .. _G.ItemFarm.state.run_id .. ')')
end

-- look 超時（護板）
function _G.ItemFarm.check_dispel_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "checking_dispel" then return end
    -- 默認重試
    _G.ItemFarm.retry_dispel_with_look(rid)
end

-- 重試 dispel + look
function _G.ItemFarm.retry_dispel_with_look(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    s.dispel_retries = s.dispel_retries + 1
    if s.dispel_retries >= 10 then
        mud.echo("⚠️ Dispel 失敗 10 次，返回儲存點...")
        s.dispel_retries = 0
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return(" .. s.run_id .. ")")
    else
        mud.echo("❌ Dispel 未生效 (" .. s.dispel_retries .. "/10)，重試...")
        s.stage = "dispelling"
        mud.timer(1.0, "_G.ItemFarm.do_dispel_and_check(" .. s.run_id .. ")")
    end
end

-- dispel + check 的 wrapper
function _G.ItemFarm.do_dispel_and_check(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    local j = _G.ItemFarm.job()
    mud.send(j.dispel_cmd)
    mud.timer(1.5, '_G.ItemFarm.check_dispel(' .. _G.ItemFarm.state.run_id .. ')')
end

-- 舊版 dispel 重試（無 dispel_indicator）
function _G.ItemFarm.retry_dispel_legacy()
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "dispelling" then return end
    local j = _G.ItemFarm.job()
    mud.send(j.dispel_cmd)
end

-- 2c. Dispel 成功後，送 buff 再攻擊
function _G.ItemFarm.buff_and_attack(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    
    -- 執行 buff 指令
    if j.buff_cmds then
        for _, cmd in ipairs(j.buff_cmds) do
            mud.send(cmd)
        end
        mud.timer(2.0, "_G.ItemFarm.do_attack(" .. _G.ItemFarm.state.run_id .. ")")
    else
        mud.timer(0.5, "_G.ItemFarm.do_attack(" .. _G.ItemFarm.state.run_id .. ")")
    end
end

-- 3. 召喚並攻擊
function _G.ItemFarm.summon_and_attack()
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "traveling" and 
       _G.ItemFarm.state.stage ~= "summoning" and 
       _G.ItemFarm.state.stage ~= "checking_status_pre_summon" then
        return
    end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.stage = "summoning"
    mud.echo("✨ [" .. j.name .. "] 召喚中... (嘗試 " .. (_G.ItemFarm.state.summon_retries + 1) .. "/3)")
    
    mud.send(j.summon_cmd)
end

-- 3. 發送攻擊前檢查 (現在改用 score)
function _G.ItemFarm.do_attack(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local s = _G.ItemFarm.state
    s.stage = "checking_status_pre_fight"
    s.last_score_time = os.time()
    mud.echo("📊 戰鬥前檢查狀態 (發送 score)...")
    mud.send("score")
    mud.send("save")
end

-- 直接開始戰鬥（跳過 score 檢查，用於召喚後）
function _G.ItemFarm.start_fighting(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.stage = "fighting"
    mud.echo("⚔️ [" .. j.name .. "] 召喚成功，直接開始攻擊！")
    send_cmds(j.attack_cmd)
end

-- 根據狀態評估是否開始戰鬥
function _G.ItemFarm.evaluate_status_and_fight()
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config

    -- 檢查 HP/MP 是否足夠戰鬥
    local j_hp_threshold = j.hp_threshold or cfg.hp_threshold
    local hp_ok = (s.max_hp == 0) or (j_hp_threshold == 0) or ((s.current_hp / s.max_hp) * 100 >= j_hp_threshold)
    local mp_ok = (s.max_mp == 0) or ((s.current_mp / s.max_mp) * 100 >= cfg.mp_threshold)
    local req_sanctuary = cfg.require_sanctuary
    if j.require_sanctuary ~= nil then req_sanctuary = j.require_sanctuary end
    local sanc_ok = not req_sanctuary or s.has_sanctuary
    
    if not hp_ok or not mp_ok then
        local reason = not hp_ok and "HP" or "MP"
        local threshold = not hp_ok and j_hp_threshold or cfg.mp_threshold
        mud.echo("⚠️ " .. reason .. " 不足，返回休息...「"
            .. "HP:" .. s.current_hp .. "/" .. s.max_hp 
            .. " MP:" .. s.current_mp .. "/" .. s.max_mp .. "」")
        s.stage = "returning"
        local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
        _G.ItemFarm.walk_path(path, "_G.ItemFarm.after_return()")
        return
    end


    if not sanc_ok then
        mud.echo("🛡️ 聖光不足，嘗試施放 'c san'...")
        mud.send("c san")
        -- 延遲 2 秒後重新檢查狀態
        mud.timer(2.0, "_G.ItemFarm.do_attack(" .. s.run_id .. ")")
        return
    end
    
    s.stage = "fighting"
    mud.echo("⚔️ [" .. j.name .. "] 狀態良好（含聖光），開始攻擊！")
    send_cmds(j.attack_cmd)
end

function _G.ItemFarm.summon_failed_too_many()
    local j = _G.ItemFarm.job()
    mud.echo("⚠️ [" .. j.name .. "] 召喚失敗 3 次，跳到下一個任務...")
    _G.ItemFarm.state.summon_retries = 0
    _G.ItemFarm.state.stage = "returning"
    
    local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
    _G.ItemFarm.walk_path(path, "_G.ItemFarm.after_summon_fail(" .. _G.ItemFarm.state.run_id .. ")")
end

-- 召喚失敗返回後，切換到下一個任務
function _G.ItemFarm.after_summon_fail(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    _G.ItemFarm.state.stage = "idle"
    _G.ItemFarm.next_job()
end

-- 返回儲存點後，休息再切換任務
function _G.ItemFarm.after_return(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    _G.ItemFarm.state.stage = "resting"
    mud.echo("💤 休息中...")
    mud.send(_G.ItemFarm.config.rest_cmd)
    mud.timer(5.0, "_G.ItemFarm.check_mp(" .. _G.ItemFarm.state.run_id .. ")")
end

-- 4. 收集戰利品
function _G.ItemFarm.loot()
    -- 戰利品階段通常由 Hook 直接觸發，不需要 run_id 檢查，
    -- 但其後續的 timer 需加上
    if not _G.ItemFarm.state.running then return end
    
    _G.ItemFarm.state.stage = "looting"
    local j = _G.ItemFarm.job()
    mud.echo("💰 收集戰利品...")
    for _, item in ipairs(j.loot_items) do
        mud.send("get " .. item .. " corpse")
    end
    if j.sac_corpse then
        mud.send("sac corpse")
    end
    mud.timer(1.0, "_G.ItemFarm.go_to_storage(" .. _G.ItemFarm.state.run_id .. ")")
end

-- 5. 前往儲存地點
function _G.ItemFarm.go_to_storage(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.stage = "storing"
    mud.echo("📦 前往儲存地點...")
    
    local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
    _G.ItemFarm.walk_path(path, "_G.ItemFarm.remove_and_drop(" .. _G.ItemFarm.state.run_id .. ")")
end

-- 6. 移除 nodrop 並丟下
function _G.ItemFarm.remove_and_drop(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    
    -- 移除 nodrop
    if j.remove_nodrop and #j.remove_nodrop > 0 then
        for _, item in ipairs(j.remove_nodrop) do
            mud.send("c 'remove n' " .. item)
        end
        mud.timer(1.5, "_G.ItemFarm.drop_items(" .. _G.ItemFarm.state.run_id .. ")")
    else
        _G.ItemFarm.drop_items(rid)
    end
end

function _G.ItemFarm.drop_items(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    for _, item in ipairs(j.loot_items) do
        mud.send("dro " .. item)
    end
    
    _G.ItemFarm.state.loot_count = _G.ItemFarm.state.loot_count + 1
    mud.echo("✅ [" .. j.name .. "] 收集完成 (第 " .. _G.ItemFarm.state.loot_count .. " 次)")
    
    mud.timer(2.0, "_G.ItemFarm.rest_and_repeat(" .. _G.ItemFarm.state.run_id .. ")")
end

-- 緊急逃脫處理
function _G.ItemFarm.emergency_escape()
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    
    if s.stage == "emergency" then
        -- 已經在逃脫中，僅嘗試 fl
        mud.send("fl")
        return
    end
    
    mud.echo("🚨 [緊急] 偵測到非預期戰鬥！嘗試逃脫並停用此任務...")
    s.stage = "emergency"
    j.disabled = true
    
    mud.send("fl")
    mud.send("recall")
end

-- 7. 休息並重複
function _G.ItemFarm.rest_and_repeat(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    _G.ItemFarm.state.stage = "resting"
    mud.echo("💤 休息中...")
    mud.send(_G.ItemFarm.config.rest_cmd)
    
    mud.timer(5.0, "_G.ItemFarm.check_mp(" .. _G.ItemFarm.state.run_id .. ")")
end

function _G.ItemFarm.check_mp(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "resting" then return end
    
    local s = _G.ItemFarm.state
    local now = os.time()

    -- 只有間隔足夠才發送 score
    if now - s.last_score_time >= _G.ItemFarm.config.score_interval then
        s.last_score_time = now
        mud.send("score")
        mud.send("save")
    end
    
    mud.timer(5.0, "_G.ItemFarm.check_mp(" .. s.run_id .. ")")
end

-- ===== Server Message Hook =====
-- 強制更新 Hook 以支援新的 clean_line 參數 (解決熱重載參數遺失問題)
_G.on_server_message = function(line, clean_line)
    -- 注意：這裡為了確保參數能正確傳遞，我們暫時不呼叫 old_hook，
    -- 除非我們能確定 old_hook 也能接收雙參數。
    -- 在此環境下直接呼叫 ItemFarm 的處理函數。
    if _G.ItemFarm and _G.ItemFarm.on_server_message then
        _G.ItemFarm.on_server_message(line, clean_line)
    end
end
_G.ItemFarm.hook_installed = true

-- 接收伺服器訊息 Hook
-- 接收伺服器訊息 Hook
function _G.ItemFarm.on_server_message(line, clean_line)
    if not _G.ItemFarm.state.running then return end
    
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    -- local clean_line = string.gsub(line, "\27%[[0-9;]*m", "") -- 已由 Rust 端傳入
    
    -- 0. 基礎狀態更新 (Walking)
    
    -- 偵測體力耗盡（只在行走中、未暫停時觸發）
    if s.walking and not s.path_paused and string.find(clean_line, "你精疲力竭了") then
        s.path_paused = true
        -- 不前進 index，下次恢復後重試同一個指令
        mud.echo("💤 體力不足，施放 refresh...")
        mud.timer(0.5, "_G.ItemFarm.recover_stamina(" .. s.run_id .. ")")
        return
    end

    if s.walking and not s.path_paused and string.find(clean_line, "你的體力逐漸地恢復") then
        _G.ItemFarm.walk_resume()
        return
    end

    -- 偵測 prompt (移動指令)
    if s.walking and not s.path_paused and string.find(clean_line, "%[出口:") then
        _G.ItemFarm.walk_advance()
        return
    end

    -- 1. 狀態機分流處理
    if s.stage == "fighting" then
        -- [戰鬥階段] 僅檢查擊殺、逃跑、目標消失
        if string.find(clean_line, "魂歸西天了") and string.find(clean_line, j.target_mob) then
            mud.echo("💀 目標已擊殺！")
            mud.timer(0.5, "_G.ItemFarm.loot()")
            return
        end
        
        if string.find(clean_line, j.target_mob) and 
           (string.find(clean_line, "逃了") or string.find(clean_line, "離開了")) then
            _G.ItemFarm.handle_mob_fled(j)
            return
        end
        
        if string.find(clean_line, "目標不在") or string.find(clean_line, "施法的目標不在") then
            _G.ItemFarm.handle_mob_missing(j)
            return
        end

    elseif s.stage == "summoning" then
        -- [召喚階段] 僅檢查成功與失敗
        if string.find(clean_line, "突然出現在你的眼前") then
            mud.echo("✅ 召喚成功！")
            s.summon_retries = 0
            -- 召喚前已檢查過狀態，直接開打
            mud.timer(0.5, "_G.ItemFarm.start_fighting(" .. s.run_id .. ")")
            return
        end
        
        if string.find(clean_line, "你失敗了") then
            s.summon_retries = s.summon_retries + 1

            if s.summon_retries >= 3 then
                mud.timer(0.5, "_G.ItemFarm.summon_failed_too_many(" .. s.run_id .. ")") -- 補上 run_id
            else
                mud.echo("❌ 召喚失敗，重試...")
                mud.timer(1.0, "_G.ItemFarm.summon_and_attack(" .. s.run_id .. ")") -- 補上 run_id
            end
            return
        end

    elseif s.stage == "searching" then
        -- [搜尋階段] 僅檢查 Quest/Locate 關鍵字
        local found = false
        if j.search_type == "quest" then
            if string.find(clean_line, "他正在這個世界中") then found = true end
        elseif j.search_type == "locate" then
            if string.find(clean_line, j.target_mob) and string.find(clean_line, "攜帶著") then found = true end
        end
        
        if found then
            mud.echo("🎯 [" .. j.name .. "] 目標存在！前往戰鬥...")
            s.found_target = true
            s.stage = "traveling"
            mud.timer(1.0, "_G.ItemFarm.go_and_fight()")
            return
        end

    elseif s.stage == "verifying_mob" then
        -- [驗證 Mob 存在]
        if string.find(clean_line, j.target_mob) and
           not string.find(clean_line, "屍體") and
           not string.find(clean_line, "corpse") then
            mud.echo("✅ 目標在場！")
            s.stage = "verified"
            _G.ItemFarm.start_dispel_or_attack()
            return
        end

    elseif s.stage == "verifying_loc" then
        -- [驗證 Mob 位置]
        if string.find(clean_line, "攜帶著") then
            mud.echo("🚫 [" .. j.name .. "] 目標在別處！永久停用此任務（需手動找回）")
            j.disabled = true
            s.stage = "returning"
            _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return()")
            return
        end

    elseif s.stage == "checking_dispel" then
        -- [檢查 Dispel 結果 (Indicator)]
        if string.find(clean_line, j.target_mob) and
           not string.find(clean_line, "屍體") and
           not string.find(clean_line, "corpse") then
            if j.dispel_indicator and string.find(clean_line, j.dispel_indicator) then
                s.stage = "dispelling"
                _G.ItemFarm.retry_dispel_with_look()
            else
                mud.echo("✅ Dispel 成功！（" .. (j.dispel_indicator or "") .. " 已消失）")
                s.dispel_retries = 0
                s.stage = "dispelled"
                mud.timer(0.5, "_G.ItemFarm.buff_and_attack()")
            end
            return
        end

    elseif s.stage == "dispelling" then
        -- [檢查 Dispel 結果 (Legacy)]
        if not j.dispel_indicator then
            if string.find(clean_line, "OK") then
                mud.echo("✅ Dispel 成功！")
                s.dispel_retries = 0
                mud.timer(0.5, "_G.ItemFarm.buff_and_attack()")
                return
            end
            if string.find(clean_line, "你失敗了") then
                _G.ItemFarm.handle_dispel_fail_legacy(s, j)
                return
            end
        end

    elseif s.stage == "emergency" then
        -- [緊急逃脫階段]
        if string.find(clean_line, "你為了保命而不顧面子從戰鬥中逃了") or
           string.find(clean_line, " recall") then
            mud.echo("✅ 成功逃離戰鬥！")
            s.stage = "idle"
            _G.ItemFarm.next_job()
            return
        end
        if string.find(clean_line, "你逃跑失敗了") then
            mud.send("fl")
            return
        end
    end

    -- 2. 全域/通用檢查 (必要時執行)

    -- 偵測體力恢復成功（refresh 生效）
    if s.path_paused and string.find(clean_line, "你的體力逐漸地恢復") then
        mud.echo("✅ 體力已恢復，繼續前進...")
        mud.timer(0.5, "_G.ItemFarm.walk_resume()")
        return
    end

    -- 非預期戰鬥偵測 (排除已在戰鬥或逃脫中)
    if s.stage ~= "fighting" and s.stage ~= "emergency" then
        if string.find(clean_line, "伺機而動") or 
           string.find(clean_line, "蓄勢待發") or
           string.find(clean_line, "身陷戰鬥中") then
            _G.ItemFarm.emergency_escape()
            return
        end
    end

    -- Score 解析 (HP/MP/Spells)
    -- 僅在相關狀態檢查，減少不必要的匹配
    if s.stage == "checking_status_pre_fight" or 
       s.stage == "checking_status_pre_summon" or 
       s.stage == "resting" then
        
        -- 生命力/精神力
        local h_cur, h_max = string.match(clean_line, "生命力:%s+(%d+)/%s+(%d+)")
        if h_cur and h_max then
            s.current_hp = tonumber(h_cur)
            s.max_hp = tonumber(h_max)
        end
        
        local m_cur, m_max = string.match(clean_line, "精神力:%s+(%d+)/%s+(%d+)")
        if m_cur and m_max then
            s.current_mp = tonumber(m_cur)
            s.max_mp = tonumber(m_max)
        end

        -- 聖光偵測
        if string.find(clean_line, "法術: '聖光'") then
            s.has_sanctuary = true
        end

        -- Score 結束行 (表頭觸發後延遲判定)
        if string.find(clean_line, "目前對你產生影響的法術或技巧有") then
            s.has_sanctuary = false -- 解析開始前重設
            
            if s.stage == "checking_status_pre_fight" then
                mud.timer(0.8, "_G.ItemFarm.evaluate_status_and_fight()")
            elseif s.stage == "checking_status_pre_summon" then
                mud.timer(0.8, "_G.ItemFarm.evaluate_status_before_summon()")
            elseif s.stage == "resting" then
                mud.timer(0.8, "_G.ItemFarm.evaluate_resting_status()")
            end
        end
    end
end

-- 輔助函數：處理目標逃跑
function _G.ItemFarm.handle_mob_fled(j)
    local s = _G.ItemFarm.state
    local mode = j.mode or "summon"
    if mode == "summon" then
        mud.echo("🏃 目標逃跑了！重新召喚...")
        s.stage = "summoning"
        s.summon_retries = 0
        mud.timer(0.5, "_G.ItemFarm.summon_and_attack()")
    else
        mud.echo("🏃 目標逃跑了！返回儲存點...")
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return()")
    end
end

-- 輔助函數：處理目標消失
function _G.ItemFarm.handle_mob_missing(j)
    local s = _G.ItemFarm.state
    local mode = j.mode or "summon"
    if mode == "summon" then
        mud.echo("❌ 目標不在這裡！重新召喚...")
        s.stage = "summoning"
        s.summon_retries = 0
        mud.timer(0.5, "_G.ItemFarm.summon_and_attack()")
    else
        mud.echo("❌ 目標不在這裡！返回儲存點...")
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return()")
    end
end

-- 輔助函數：處理 Dispel 失敗 (Legacy)
function _G.ItemFarm.handle_dispel_fail_legacy(s, j)
    s.dispel_retries = s.dispel_retries + 1
    if s.dispel_retries >= 10 then
        mud.echo("⚠️ Dispel 失敗 10 次，返回儲存點...")
        s.dispel_retries = 0
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return()")
    else
        mud.echo("❌ Dispel 失敗 (" .. s.dispel_retries .. "/10)，重試...")
        mud.timer(1.0, "_G.ItemFarm.retry_dispel_legacy()")
    end
end

-- 休息階段的狀態評估
function _G.ItemFarm.evaluate_resting_status()
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config
    
    if s.stage ~= "resting" then return end

    -- 在休息階段檢查是否可以起床
    local hp_pct = (s.max_hp > 0) and math.floor((s.current_hp / s.max_hp) * 100) or 100
    local mp_pct = (s.max_mp > 0) and math.floor((s.current_mp / s.max_mp) * 100) or 100
    
    local hp_threshold = j.hp_threshold or cfg.hp_threshold
    local hp_ok = (hp_threshold == 0) or (hp_pct >= hp_threshold)
    local mp_ok = (mp_pct >= cfg.mp_threshold)
    local req_sanctuary = cfg.require_sanctuary
    if j.require_sanctuary ~= nil then req_sanctuary = j.require_sanctuary end
    local sanc_ok = not req_sanctuary or s.has_sanctuary



    -- 如果 HP 不足且有恢復指令
    if not hp_ok and j.hp_recover_cmd then
        mud.echo("⚡ HP 不足，站立並執行恢復: " .. j.hp_recover_cmd)
        mud.send("wa")
        mud.send(j.hp_recover_cmd)
        mud.send(cfg.rest_cmd)
        return
    end

    -- 如果聖光不足且 MP/HP 足夠，嘗試補上
    if hp_ok and mp_ok and not sanc_ok then
        mud.echo("🛡️ 聖光不足 (休息中)，起身施放 'c san'...")
        mud.send("wa")
        mud.send("c san")
        mud.send(cfg.rest_cmd)
        return
    end

    if hp_ok and mp_ok and sanc_ok then
        mud.echo("✅ 狀態已回滿（含聖光） (HP:" .. s.current_hp .. " MP:" .. s.current_mp .. ")，繼續下一輪...")
        s.stage = "idle"
        s.jobs_checked = 0
        mud.send("wa")
        mud.timer(1.0, "_G.ItemFarm.search()")
    end
end

-- 初始化
_G.ItemFarm.init()
