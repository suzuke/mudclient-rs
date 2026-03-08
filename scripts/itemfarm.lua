-- ============================================================
-- ItemFarm v2.1 - 自動打怪收集物品 (多任務輪替)
-- ============================================================
-- 模式：
--   summon  = 前往安全點 → 召喚 → 攻擊
--   direct  = 前往怪物處 → (dispel → buff) → 攻擊
--   charm   = 前往怪物處 → 魅惑 (attack_cmd) → 帶領到擊殺點 (path_to_kill) → 等待被殺 / 撿取
-- 流程：
-- 1. 查詢當前任務的怪物是否重生
-- 2. 未重生 → 跳到下一個任務；全部都沒重生 → 等待後重新輪替
-- 3. 重生 → 依模式擊殺 → 收集戰利品 → 儲存 → 休息 → 重複
-- ============================================================

_G.ItemFarm = _G.ItemFarm or {}

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("ItemFarm cannot load dependency: " .. name)
end

local MudCombat = require_module("MudCombat")
local MudLoot = require_module("MudLoot")
local MudNav = require_module("MudNav")
local MudUtils = require_module("MudUtils")

-- local mud = mud -- 避免快取 userdata
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
    hp_threshold = 90,        -- 全局 HP 門檻 (0 表示預設不檢查)
    poll_interval = 30,      -- 全部未重生時的等待秒數
    rest_cmd = "sleep",          -- 休息指令
    score_interval = 20,     -- score 指令最小間隔 (秒)
    show_echo = true,        -- 是否顯示非關鍵訊息
}

-- ===== 共用路徑 =====
local function extend_path(base, extra)
    local p = {}
    for i, v in ipairs(base) do p[i] = v end
    for _, v in ipairs(extra) do p[#p + 1] = v end
    return p
end

-- recall → 博物館入口 → look painting → 吉雅尼斯城 → 主要廣場
local path_to_painting_area = {
    {cmd="recall", id="4e9c9dd2418fa5c52e762d52985dfca6fe1d77cd111c87536d3211df7cf5ca2e"},
    {cmd="w", id="478ffe9f12a30d704186f327ca56a85531b46db421602509b46fe58eb6c267c5"},
    {cmd="w", id="349965591ea7cea0ca7de23a02a1cdc517fe5d1617867d0cb8d4623c72af7dbd"},
    {cmd="w", id="314bd5656517c827bebea9e72a871802325927e2241ee17c5f4ed37b420f39a6"},
    {cmd="s", id="582cf9bb9b3444959f6ac8882c7e6f6174599a42efa746ccad882be7bfd8bfaa"},
    {cmd="s", id="b370fe0b35b66d61dd7c1b38070c1dbcea24550aabad37e266148b7244204459"},
    {cmd="s", id="5530f4e241b3cd903a4cc158d4732764c974e1bcb554ce97169140eecda97a85"},
    {cmd="e", id="97f8fe848f5492717e2b12e0538552c62937548236d20a028f7cc1ebaedb18b8"},
    {cmd="look painting", id="e913d4e99d70ba89895dab53d22aa6669d5c585ffb066eb241cb2abedace31b3"},
    {cmd="s", id="eef1fdd15511ea3c48793299ca947aa4de44be5a095cd7194a4e103740fc2e2c"},
}

-- ===== 任務列表 =====
-- search_type: "quest" (偵測「他正在這個世界中」) / "locate" (偵測「攜帶著」)
-- mode: "summon" (召喚後攻擊) / "direct" (直接到場攻擊)
-- buffs: { {cmd="施法指令", indicator="score 中的法術名", fade_msg="自定義消散訊息"}, ... }
-- pre_travel_cmd: "移動前執行指令" (例如 "c fly")
_G.ItemFarm.jobs = {
    {
        name = "商務間諜",
        mode = "summon",             -- summon 或 direct
        search_type = "locate",
        search_cmd = "c loc grating",
        target_mob = "商務間諜",
        summon_cmd = "c sum spy",
        attack_cmd = "c flame spy",
        path_to_mob = "recall;2n;2e",
        path_to_storage = "recall;3n;e",
        loot_items = {"anesthetic", "grating"},
        remove_nodrop = {"anesthetic", "grating"},
        sac_corpse = true,
        disabled = true,
    },
    {
        name = "街頭混混",
        mode = "summon",
        search_type = "quest",
        search_cmd = "q 28.boy",
        target_mob = "街頭混混",
        summon_cmd = "c sum boy",
        attack_cmd = "c flame boy",
        path_to_mob = "recall",
        path_to_storage = "recall;3n;e",
        loot_items = {"take"},
        remove_nodrop = {},
        sac_corpse = true,
        disabled = true,
    },
    {
        name = "某校生",
        mode = "summon",
        search_type = "locate",
        search_cmd = "c loc id",
        target_mob = "某校生",
        summon_cmd = "c sum student",
        attack_cmd = "c nu student;c fl student",
        path_to_mob = "recall",
        path_to_storage = "recall;3n;e",
        loot_items = {"id"},
        remove_nodrop = {},
        sac_corpse = true,
        disabled = false,
    },
    {
        name = "不動明王",
        mode = "direct",
        search_type = "quest",
        search_cmd = "q 6.sentinel",
        target_mob = "不動明王",
        attack_cmd = "c star;c star;c star",
        dispel_cmd = "c 'dispel m' sentinel",
        dispel_indicators = {"白色聖光"},    -- 只要其中一個在場就繼續 dispel
        hp_threshold = 100,               -- 特定怪物才檢查血量
        hp_recover_cmd = "c heal",         -- 自定義恢復 HP 的指令
        buffs = {
            { cmd = "c sa",  indicator = "聖光", fade_msg = "你四周的白色聖光消散了" },
            { cmd = "c pro", indicator = "聖佑術", fade_msg = "你感覺到失去上天的護佑." },
            { cmd = "c b",   indicator = "女神庇祐術", fade_msg = "你覺得你的好運已經結束了." }
        },
        dispel_max_retries = 15,     -- 自定義重試次數
        pre_travel_cmd = "c inv",  -- 隱身
        path_to_mob = "recall;3w;4s;ta wizard help;7w;7n;6u;7n",
        path_to_storage = "recall;3n;e",
        loot_items = {"sword", "potato", "hamburg"},
        remove_nodrop = {},
        sac_corpse = true,
        disabled = true,
    },
    {
    -- Xiulou slash /11swn3e2ne3ne2nu4ne
        name = "闇の一族幫員",
        mode = "direct",
        search_type = "quest",
        search_cmd = "q clan_member",
        target_mob = "闇の一族幫員",
        attack_cmd = "c nu clan_member;c fl clan_member",
        pre_travel_cmd = "c inv",  -- 隱身
        path_to_mob = "recall;11s;w;n;3e;2n;e;3n;e;2n;u;4n;e",
        path_to_storage = "recall;3n;e",
        loot_items = {"Xiulou"},
        remove_nodrop = {},
        sac_corpse = true,
        disabled = true,
    },
    {
        name = "天堂守護者麥倫．薩爾達",
        mode = "direct",
        search_type = "quest",
        search_cmd = "q 2.paradiser",
        target_mob = "麥倫．薩爾達",
        attack_cmd = "c star;c star;c star;",
        dispel_cmd = "c 'dispel m' paradiser",
        dispel_indicators = {"白色聖光"},    -- 只要其中一個在場就繼續 dispel
        hp_threshold = 100,               -- 特定怪物才檢查血量
        hp_recover_cmd = "c heal",         -- 自定義恢復 HP 的指令
        buffs = {
            { cmd = "c sa",  indicator = "聖光", fade_msg = "你四周的白色聖光消散了" },
            { cmd = "c pro", indicator = "聖佑術", fade_msg = "你感覺到失去上天的護佑." },
            { cmd = "c b",   indicator = "女神庇祐術", fade_msg = "你覺得你的好運已經結束了." }
        },
        dispel_max_retries = 15,     -- 自定義重試次數
        path_to_mob = "recall;3w;2s;5e;2s;e;op e;e",
        path_to_storage = "recall;3n;e",
        loot_items = {"wisdom"},
        remove_nodrop = {},
        sac_corpse = true,
        disabled = true,
    },

    -- 動靈帽 (Charm 模式)
    {
        name = "動靈帽",
        mode = "charm",
        search_type = "locate",
        search_cmd = "c loc mind",
        target_mob = {"一位魔法見習生", "魔法見習生"},
        attack_cmd = "c charm student", -- 魅惑指令

        path_to_mob = extend_path(path_to_painting_area, {
            {cmd="w", id="17d58425b0bfaeb6cd94043280833333ed3aa5a503b0b094fc13e877a7fce6cb"},
            {cmd="w", id="de0bc88cac4c3db67d96141608ab2d39af185a9f6346f3f76c191b93f1bd7909"},
            {cmd="s", id="a0bdbe8419a511cf0d91f90a708e8be8aca8fa6f48d5cd1ca527e5821015edf8"},
        }),
        -- 魅惑後帶往擊殺點的路徑
        path_to_kill = "or all hi;or all recall;recall;n;n;n;e", 
        -- 抵達擊殺點後，若需強制目標卸下裝備可設定此項
        kill_action = "or all drop hat;w;s;s;s;w;w;n",
        path_to_storage = "recall;3n;e",
        loot_items = {},
        remove_nodrop = {},
        sac_corpse = false, -- 目標可能被系統清掉，不需要 sac
        disabled = false, -- 預設先停用，修改好參數後可把這行移除
    },
    {
        name = "詛咒之劍",
        mode = "summon",
        search_type = "quest",
        search_cmd = "q 17.traveller",
        summon_cmd = "c summon traveller",
        target_mob = {"一位四處旅行的","旅人"},
        attack_cmd = "c fire traveller",
        path_to_mob = path_to_painting_area,
        path_to_storage = "recall;3n;e",
        loot_items = {"curse"},
        remove_nodrop = {},
        sac_corpse = true,
        disabled = false,
    },
    -- {
    --     name = "銀行職員 (測試 safe_charm)",
    --     mode = "charm",
    --     search_type = "quest",
    --     search_cmd = "q servant",
    --     target_mob = {"西裝筆挺", "銀行職員"},
    --     attack_cmd = "c charm servant",
    --     path_to_mob = {
    --         {cmd="recall", id="4e9c9dd2418fa5c52e762d52985dfca6fe1d77cd111c87536d3211df7cf5ca2e"},
    --         {cmd="n", id="89ab35d8c4dfacdb66314ae3b3d2700053e39f552fe8f140aa14941ad9da7574"},
    --         {cmd="n", id="60f099d91883b289ba84f1745d7adf38cf4c23b1886b4d58884619f9a1d9413c"},
    --         {cmd="n", id="ea3efa3229d62ea33d8dade3608e6672a52af3ee03747d70e5e9d41efb23a1ab"},
    --         {cmd="n", id="67936504a24314c48830ecfb6304301615bdf0e3f6723e09aa2c1b65944c26da"},
    --         {cmd="e", id="05879f4b72fb468bb4f31c9112a969d88bfc8e011f7f19cbef61c2bc2dcb9c4c"},
    --     },
    --     path_to_kill = "w;s;s;s;e", 
    --     kill_action = "or all drop all",
    --     path_to_storage = "recall",
    --     loot_items = {},
    --     remove_nodrop = {},
    --     sac_corpse = false,
    --     disabled = false,
    -- },
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
    charm_retries = 0,
    current_job = 1,       -- 當前任務索引
    jobs_checked = 0,      -- 本輪已檢查的任務數
    last_score_time = 0,   -- 上次發送 score 的時間
    active_spells = {},    -- 當前身上的法術 { ["法術名"] = 小時 }
    -- 路徑佇列（prompt 驅動）
    path_queue = {},
    path_index = 0,
    path_callback = nil,
    walking = false,       -- 是否正在行走中
}

-- 檢查 run_id 是否有效
local function check_run(run_id)
    if not run_id then return true end
    return tonumber(run_id) == tonumber(_G.ItemFarm.state.run_id)
end

-- ===== 訊息輸出輔助 =====

-- 普通訊息 (受 show_echo 控制)
function _G.ItemFarm.echo(msg)
    if _G.ItemFarm.config.show_echo then
        mud.echo(msg)
    end
end

-- 強制訊息 (不受 show_echo 控制，用於啟動、停止、報錯)
function _G.ItemFarm.echo_force(msg)
    mud.echo(msg)
end

-- 切換顯示開關
function _G.ItemFarm.toggle_echo()
    local cfg = _G.ItemFarm.config
    cfg.show_echo = not cfg.show_echo
    local status = cfg.show_echo and "開啟" or "關閉"
    _G.ItemFarm.echo_force("📢 ItemFarm 訊息顯示已 " .. status)
end

-- ===== 輔助函數 =====
function _G.ItemFarm.job()
    return _G.ItemFarm.jobs[_G.ItemFarm.state.current_job]
end

-- 匹配 target_mob（支援 string 或 table）
local function match_target(line, target_mob)
    if type(target_mob) == "table" then
        for _, kw in ipairs(target_mob) do
            if string.find(line, kw, 1, true) then return true end
        end
        return false
    else
        return string.find(line, target_mob, 1, true) ~= nil
    end
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

-- ===== Timer Helper (防止舊 Timer 觸發) =====

-- 安全計時器：自動注入 run_id 作為最後一個參數
-- 使用方式：ItemFarm.safe_timer(秒數, "函數名", 參數1, 參數2, ...)
-- 例如：ItemFarm.safe_timer(2.0, "_G.ItemFarm.search")
function _G.ItemFarm.safe_timer(seconds, func_name, ...)
    local s = _G.ItemFarm.state
    if not s.running then return end

    local args = {...}
    table.insert(args, s.run_id) -- 自動補上 run_id

    -- 序列化參數
    local serialized_args = {}
    for _, v in ipairs(args) do
        if type(v) == "string" then
            table.insert(serialized_args, string.format("%q", v))
        else
            table.insert(serialized_args, tostring(v))
        end
    end

    local code = func_name .. "(" .. table.concat(serialized_args, ", ") .. ")"
    mud.timer(seconds, code)
end

-- 檢查是否有缺失的 Buff
-- 回傳：buff 物件 (若有缺), nil (全滿)
function _G.ItemFarm.get_missing_buff(rid)
    if not check_run(rid) then return nil end
    if not _G.ItemFarm.state.running then return nil end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    if not j.buffs or #j.buffs == 0 then return nil end
    
    for _, b in ipairs(j.buffs) do
        local hours = s.active_spells[b.indicator]
        
        -- 安全判定：若無此 Buff，或時數 == 0 (即將過期)，均視為「需要補充」
        -- 注意：若時數為 -1 代表永久或特殊時效，不應視為缺失
        if not hours or hours == 0 then
            return b
        end
    end
    
    return nil
end

-- 檢查並補足 Buff (通用版：直接施放)
-- 回傳：true (全部 Buff 已到位), false (補法中), "waiting" (等消散中)
function _G.ItemFarm.check_and_apply_buffs(rid)
    local s = _G.ItemFarm.state
    local b = _G.ItemFarm.get_missing_buff(rid)
    if not b then return true end
    
    local hours = s.active_spells[b.indicator]
    if hours and hours <= 0 then
        -- 0 小時狀態：等待消散，暫不施放（因為施放會失敗）
        _G.ItemFarm.echo("⌛ Buff [" .. b.indicator .. "] 即將到期 (0hr)，等待消散中...")
        return "waiting"
    end

    _G.ItemFarm.echo("✨ 補 Buff: " .. b.indicator .. " (" .. b.cmd .. ")")
    mud.send(b.cmd)
    return false
end


-- ===== 移動系統 (使用 MudNav) =====
function _G.ItemFarm.walk_path(str, callback_name)
    local s = _G.ItemFarm.state
    if not s.running then return end
    
    -- 回調封裝：處理字串型回調
    local cb = function()
        if not s.running then return end
        if type(callback_name) == "string" then
             local func = _G.ItemFarm[callback_name:match("ItemFarm%.(.+)") or callback_name]
             if func then func(s.run_id) end
        elseif type(callback_name) == "function" then
             callback_name(s.run_id)
        end
    end
    
    s.walking = true -- 標記為行走中，用於 Hook 過濾
    MudNav.walk(str, function()
        s.walking = false
        cb()
    end)
end

function _G.ItemFarm.recover_stamina(rid)
    -- MudNav 已經內建 recover_stamina，這裡保留空殼或移除，
    -- 但若其他地方直接呼叫此函數，則需保留。
    -- 目前主要由 MudNav 處理。
    if not check_run(rid) then return end
    mud.send("c ref")
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
    mud.echo("    ItemFarm.toggle_echo() - 切換是否顯示詳細日誌")
    mud.echo("    ItemFarm.toggle_job(n) - 暫停/恢復第 n 個任務")
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
    s.run_id = MudUtils.get_new_run_id()
    s.stage = "idle"
    s.loot_count = 0
    s.summon_retries = 0
    s.current_job = 1
    s.jobs_checked = 0
    s.active_spells = {} -- 重置法術清單
    
    local j = _G.ItemFarm.job()
    mud.echo("🎯 開始自動收集 (" .. #_G.ItemFarm.jobs .. " 個任務)")
    mud.emit("itemfarm_started", {jobs = #_G.ItemFarm.jobs})
    MudUtils.start_log("itemfarm")
    
    -- 註冊並觸發物品檢查（非同步），完成後呼叫 after_inventory_checked
    MudUtils.register_quest("ItemFarm", _G.ItemFarm.stop)
    mud.echo("🔍 正在檢查隨身物品...")
    MudUtils.start_inventory_check("_G.ItemFarm.after_inventory_checked(tostring(" .. s.run_id .. "))")
end

-- 物品檢查通過後，真正開始掛機流程
function _G.ItemFarm.after_inventory_checked(rid)
    -- 注意參數傳遞問題，這裡轉成數字比較安全
    local run_id = tonumber(rid)
    if not check_run(run_id) then return end
    if not _G.ItemFarm.state.running then return end

    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    mud.echo("✅ 物品檢查通過，開始執行任務")
    mud.echo("   當前任務: [" .. s.current_job .. "] " .. j.name)
    _G.ItemFarm.search(s.run_id)
end

function _G.ItemFarm.stop()
    if not _G.ItemFarm.state.running then return end -- 防止重複呼叫
    _G.ItemFarm.state.running = false
    _G.ItemFarm.state.stage = "idle"
    mud.echo("🛑 已停止自動收集")
    mud.emit("itemfarm_stopped", {loot_count = _G.ItemFarm.state.loot_count})
    MudUtils.stop_log()
    mud.echo("   本次收集: " .. _G.ItemFarm.state.loot_count .. " 次")
end

function _G.ItemFarm.status()
    local s = _G.ItemFarm.state
    _G.ItemFarm.echo_force("📊 ItemFarm 狀態:")
    _G.ItemFarm.echo_force("   運行中: " .. (s.running and "是" or "否"))
    _G.ItemFarm.echo_force("   階段: " .. s.stage)
    _G.ItemFarm.echo_force("   收集次數: " .. s.loot_count)
    if s.running then
        local j = _G.ItemFarm.job()
        _G.ItemFarm.echo_force("   當前任務: [" .. s.current_job .. "] " .. j.name)
    end
    _G.ItemFarm.echo_force("   任務列表:")
    for i, j in ipairs(_G.ItemFarm.jobs) do
        local marker = (i == s.current_job and s.running) and " ◀" or ""
        local disabled = j.disabled and " [已停用]" or ""
        _G.ItemFarm.echo_force("     [" .. i .. "] " .. j.name .. disabled .. marker)
    end
end

-- 切換任務啟用/停用
function _G.ItemFarm.toggle_job(index)
    local j = _G.ItemFarm.jobs[tonumber(index)]
    if not j then
        _G.ItemFarm.echo_force("⚠️ 找不到任務 ID: " .. tostring(index))
        return
    end
    j.disabled = not j.disabled
    local status = j.disabled and "停用" or "啟用"
    _G.ItemFarm.echo_force("🔧 任務 [" .. index .. "] " .. j.name .. " 已" .. status)
end

-- ===== 任務輪替 =====
function _G.ItemFarm.next_job()
    local s = _G.ItemFarm.state
    
    -- 計算有多少啟用的任務
    local active_count = 0
    for _, j in ipairs(_G.ItemFarm.jobs) do
        if not j.disabled then active_count = active_count + 1 end
    end
    
    if active_count == 0 then
        _G.ItemFarm.echo_force("⚠️ 所有任務已停用，停止運行")
        _G.ItemFarm.stop()
        return
    end

    -- 只有當前任務是啟用的，跳過時才算「檢查過」
    local original_j = _G.ItemFarm.job()
    if not original_j.disabled then
        s.jobs_checked = s.jobs_checked + 1
    end
    
    if s.jobs_checked >= active_count then
        s.jobs_checked = 0
        s.stage = "waiting"
        _G.ItemFarm.echo("⏳ 所有目標皆未重生，" .. _G.ItemFarm.config.poll_interval .. " 秒後重新輪替...")
        mud.send(_G.ItemFarm.config.rest_cmd)
        _G.ItemFarm.safe_timer(_G.ItemFarm.config.poll_interval, "_G.ItemFarm.search")
        return
    end
    
    -- 跳到下一個未停用的任務
    local total = #_G.ItemFarm.jobs
    for _ = 1, total do
        s.current_job = (s.current_job % total) + 1
        local j = _G.ItemFarm.job()
        if not j.disabled then
            _G.ItemFarm.echo("🔄 切換任務: [" .. s.current_job .. "] " .. j.name)
            mud.emit("itemfarm_job_changed", {index = s.current_job, name = j.name})
            s.stage = "idle"
            _G.ItemFarm.safe_timer(1.0, "_G.ItemFarm.search")
            return
        end
    end

    -- 所有任務都停用
    mud.echo("⚠️ 所有任務已停用")
    _G.ItemFarm.stop()
end

-- ===== 狀態機各階段處理函數 =====

-- 1. 搜尋階段 (Searching)
function _G.ItemFarm.search(rid)
    if not check_run(rid) then return end
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
    
    _G.ItemFarm.echo("🔍 [" .. j.name .. "] 查詢目標...")
    if j.search_type ~= "quest" then
        mud.send("wa")
    end
    mud.send(j.search_cmd)
    
    -- 超時：3 秒後未偵測到 → 視為未重生
    _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm.search_timeout")
end

function _G.ItemFarm.search_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "searching" then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.search_count = _G.ItemFarm.state.search_count + 1
    _G.ItemFarm.echo("❌ [" .. j.name .. "] 目標未重生")
    
    -- 跳到下一個任務
    _G.ItemFarm.next_job()
end

-- 2. 移動階段 (Traveling)
function _G.ItemFarm.go_and_fight()
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local mode = j.mode or "summon"
    _G.ItemFarm.state.stage = "traveling"
    _G.ItemFarm.state.jobs_checked = 0  -- 重置輪替計數
    _G.ItemFarm.echo("🚶 [" .. j.name .. "] 前往目標位置...")
    mud.send("wa")
    
    -- [NEW] 執行預先準備指令 (例如上加速、飛行等)
    -- 此類指令不在 walk_path 內執行，避免破壞 Prompt 驅動機制
    if j.pre_travel_cmd then
        _G.ItemFarm.echo("⚡ 執行預備指令: " .. j.pre_travel_cmd)
        send_cmds(j.pre_travel_cmd)
    end

    if mode == "direct" then
        callback = "_G.ItemFarm.engage_direct"
    elseif mode == "charm" then
        callback = "_G.ItemFarm.engage_charm"
    else
        -- 召喚前先檢查狀態
        callback = "_G.ItemFarm.check_status_before_summon"
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
    _G.ItemFarm.echo("📊 召喚前檢查狀態 (發送 score)...")
    mud.send("rep")
    mud.send("score aff")
    mud.send("save")
end

-- 評估召喚前狀態
function _G.ItemFarm.evaluate_status_before_summon(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    
    -- 階段門護：防止計時器重複觸發
    if s.stage ~= "checking_status_pre_summon" then return end
    
    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config

    local j_hp_threshold = j.hp_threshold or cfg.hp_threshold
    local j_mp_threshold = j.mp_threshold or cfg.mp_threshold

    -- 邏輯修正：max 為 0 表示尚未獲取狀態，此時應視為「不 OK」
    local hp_ok = (s.max_hp > 0) and ((j_hp_threshold == 0) or ((s.current_hp / s.max_hp) * 100 >= j_hp_threshold))
    local mp_ok = (s.max_mp > 0) and ((j_mp_threshold == 0) or ((s.current_mp / s.max_mp) * 100 >= j_mp_threshold))

    if not hp_ok or not mp_ok then
        local reason = not hp_ok and "HP" or "MP"
        local threshold = not hp_ok and j_hp_threshold or j_mp_threshold
        _G.ItemFarm.echo("⚠️ " .. reason .. " 不足 (" .. threshold .. "% 門檻)，返回休息...")
        s.stage = "returning"
        local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
        _G.ItemFarm.walk_path(path, "_G.ItemFarm.after_return")
        return
    end

    -- 智慧 Buff 檢查
    local buff_status = _G.ItemFarm.check_and_apply_buffs(s.run_id)
    if buff_status == true then
        _G.ItemFarm.echo("✅ 狀態與 Buff 良好，開始召喚！")
        _G.ItemFarm.summon_and_attack(s.run_id)
    elseif buff_status == "waiting" then
        -- 等待消散中：30 秒保底檢查，其餘靠 Hook
        s.stage = "buffing" -- 防止 Ok. 觸發重複檢查
        _G.ItemFarm.safe_timer(30.0, "_G.ItemFarm.check_status_before_summon")
    else
        -- 補 Buff 中：2 秒後再次檢查
        s.stage = "buffing" -- 防止 Ok. 觸發重複檢查
        _G.ItemFarm.safe_timer(2.0, "_G.ItemFarm.check_status_before_summon")
    end
end

-- 2b. 直接交戰模式（到場 → 驗證 mob → dispel → buff → 攻擊）
function _G.ItemFarm.engage_direct(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "traveling" then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    -- [修正] 檢查當前房間是否正確
    local last_node = j.path_to_mob[#j.path_to_mob]
    local target_id = j.target_room_id or (type(last_node) == "table" and last_node.id)
    if target_id then
        local current_id = mud.get_current_room_id()
        if current_id ~= target_id then
            _G.ItemFarm.echo("📍 位置不正確 (目前: " .. tostring(current_id) .. ", 預期: " .. tostring(target_id) .. ")，重新導航中...")
            _G.ItemFarm.go_and_fight()
            return
        end
    end

    -- 先 look 確認 mob 是否在場
    s.stage = "verifying_mob"
    _G.ItemFarm.echo("🔍 [​" .. j.name .. "] 確認目標是否在場...")
    mud.send("l")
    -- 超時 3 秒 → mob 不在
    _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm.verify_mob_timeout")
end

-- 2c. 魅惑模式（到場 → 驗證 mob → charm → lead_to_kill）
function _G.ItemFarm.engage_charm(rid)
    _G.ItemFarm.echo_force("[DEBUG] engage_charm called with rid=" .. tostring(rid))
    if not check_run(rid) then 
        _G.ItemFarm.echo_force("[DEBUG] check_run failed. rid=" .. tostring(rid) .. " MudUtils.run_id=" .. tostring(MudUtils.run_id))
        return 
    end
    if not _G.ItemFarm.state.running then 
        _G.ItemFarm.echo_force("[DEBUG] state.running is false")
        return 
    end
    if _G.ItemFarm.state.stage ~= "traveling" and _G.ItemFarm.state.stage ~= "retry_charming" then 
        _G.ItemFarm.echo_force("[DEBUG] stage mismatch: " .. _G.ItemFarm.state.stage)
        return 
    end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    -- [修正] 檢查當前房間是否正確
    local last_node = j.path_to_mob[#j.path_to_mob]
    local target_id = j.target_room_id or (type(last_node) == "table" and last_node.id)
    if target_id then
        local current_id = mud.get_current_room_id()
        if current_id ~= target_id then
            _G.ItemFarm.echo("📍 位置不正確 (目前: " .. tostring(current_id) .. ", 預期: " .. tostring(target_id) .. ")，重新導航中...")
            _G.ItemFarm.go_and_fight()
            return
        end
    end

    s.stage = "verifying_charm_mob"
    _G.ItemFarm.echo("🔍 [​" .. j.name .. "] (Charm) 確認目標是否在場...")
    _G.ItemFarm.safe_timer(0.1, "_G.ItemFarm.send_look")
    _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm.verify_mob_timeout")
end

function _G.ItemFarm.send_look(rid)
    if not check_run(rid) then return end
    mud.send("l")
end


-- mob 不在場 → 用 search_cmd 確認是死亡還是迷路
function _G.ItemFarm.verify_mob_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "verifying_mob" and _G.ItemFarm.state.stage ~= "verifying_charm_mob" then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    _G.ItemFarm.echo("❓ [​" .. j.name .. "] 目標不在場，查詢狀態...")
    s.stage = "verifying_loc"
    mud.send(j.search_cmd)
    -- 超時 3 秒 → mob 已死
    _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm.verify_loc_timeout")
end

-- search_cmd 超時 → mob 已死，返回休息
function _G.ItemFarm.verify_loc_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "verifying_loc" then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.echo("💠 [​" .. j.name .. "] 目標已死亡，返回休息等待重生...")
    _G.ItemFarm.state.stage = "returning"
    _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return")
end

-- mob 驗證通過後，開始 dispel 或直接攻擊
function _G.ItemFarm.start_dispel_or_attack(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    if j.dispel_cmd and (j.dispel_indicators and #j.dispel_indicators > 0) then
        -- 需要 dispel：發送 dispel + look 來檢查
        s.stage = "dispelling"
        s.dispel_retries = 0
        _G.ItemFarm.echo("🔮 [" .. j.name .. "] Dispel 中...")
        mud.send(j.dispel_cmd)
        _G.ItemFarm.safe_timer(1.5, "_G.ItemFarm.check_dispel")
    else
        -- 不需要 dispel (或未設定偵測關鍵字)
        _G.ItemFarm.buff_and_attack(s.run_id)
    end
end

-- Dispel 後發送 look 檢查 indicator
function _G.ItemFarm.check_dispel(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "dispelling" then return end
    
    local s = _G.ItemFarm.state
    s.stage = "checking_dispel"
    mud.send("l")
    _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm.check_dispel_timeout")
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
    local max_retries = j.dispel_max_retries or 10
    if s.dispel_retries >= max_retries then
        _G.ItemFarm.echo("⚠️ Dispel 失敗 " .. max_retries .. " 次，返回儲存點...")
        s.dispel_retries = 0
        _G.ItemFarm.state.stage = "returning"
        local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
        _G.ItemFarm.walk_path(path, "_G.ItemFarm.after_return")
    else
        _G.ItemFarm.echo("❌ Dispel 未生效 (" .. s.dispel_retries .. "/" .. max_retries .. ")，重試...")
        s.stage = "dispelling"
        _G.ItemFarm.safe_timer(1.0, "_G.ItemFarm.do_dispel_and_check")
    end
end

-- dispel + check 的 wrapper
function _G.ItemFarm.do_dispel_and_check(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    local j = _G.ItemFarm.job()
    mud.send(j.dispel_cmd)
    -- 這裡計時器應該呼叫 check_dispel 並帶上傳進來的 rid
    _G.ItemFarm.safe_timer(1.5, "_G.ItemFarm.check_dispel")
end

-- 2c. Dispel 成功後，送 buff 再攻擊
function _G.ItemFarm.buff_and_attack(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    
    -- 執行智慧 Buff 檢查
    if not _G.ItemFarm.check_and_apply_buffs(rid) then
        -- 補法完成後會再被 score 觸發檢查，或是這裡加一個定時器重試
        _G.ItemFarm.safe_timer(2.0, "_G.ItemFarm.do_attack")
    else
        _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.do_attack")
    end
end

-- 3. 召喚階段 (Summoning)
function _G.ItemFarm.summon_and_attack(rid)
    -- 注意：此函數可能由 MudCombat 重試機制呼叫，需檢查 rid
    if rid and not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local s = _G.ItemFarm.state
    -- 狀態檢查放寬，允許從 summoning 重入 (retry)
    if s.stage ~= "traveling" and 
       s.stage ~= "summoning" and 
       s.stage ~= "checking_status_pre_summon" then
        return
    end
    
    local j = _G.ItemFarm.job()
    s.stage = "summoning"
    _G.ItemFarm.echo("✨ [" .. j.name .. "] 召喚中...")
    
    MudCombat.safe_summon(j.target_mob, j.summon_cmd, {
        max_retries = 3,
        retry_delay = 2.0,
        verify_delay = 1.0
    }, function()
        -- Success
        _G.ItemFarm.start_fighting(_G.ItemFarm.state.run_id)
    end, function()
        -- Fail
        _G.ItemFarm.summon_failed_too_many()
    end)
end

-- 3a. 魅惑階段 (Charming & Leading)
function _G.ItemFarm.do_charm_and_lead(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    s.stage = "charming"
    _G.ItemFarm.echo("💖 [" .. j.name .. "] 施放魅惑中 (第 " .. (s.charm_retries + 1) .. " 次)...")
    send_cmds(j.attack_cmd)
    
    -- 等待確認魅惑成功 (由 hook 攔截 "開始跟隨你了" 或 超時重試)
    _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm.check_charm_timeout")
end

function _G.ItemFarm.check_charm_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "charming" then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.handle_charm_failed(j, "timeout")
end

function _G.ItemFarm.handle_charm_failed(j, reason)
    local s = _G.ItemFarm.state
    s.charm_retries = s.charm_retries + 1
    
    if s.charm_retries > 3 then
        _G.ItemFarm.echo("🚫 [" .. j.name .. "] 魅惑失敗達上限，放棄並返回儲存點...")
        s.charm_retries = 0
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return")
        return
    end
    
    if reason == "combat" then
        _G.ItemFarm.echo("⚠️ [" .. j.name .. "] 魅惑失敗並進入戰鬥，嘗試逃脫重試中...")
        s.stage = "fleeing_for_charm"
        mud.send("fl")
    else
        _G.ItemFarm.echo("⚠️ [" .. j.name .. "] 魅惑似乎失敗或無回應，重試中...")
        _G.ItemFarm.do_charm_and_lead(s.run_id)
    end
end


function _G.ItemFarm.lead_to_kill_zone(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    s.stage = "leading_to_kill"
    _G.ItemFarm.echo("🚶‍♂️ [" .. j.name .. "] 魅惑成功，帶領目標前往擊殺點...")
    _G.ItemFarm.walk_path(j.path_to_kill, "_G.ItemFarm.arrive_at_kill_zone")
end

function _G.ItemFarm.arrive_at_kill_zone(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    local s = _G.ItemFarm.state
    
    s.stage = "waiting_for_kill"
    _G.ItemFarm.echo("⚔️ [" .. j.name .. "] 已抵達擊殺點，等待目標被擊殺或釋放道具...")
    
    -- [優化] 補充體力，避免 kill_action 中的導航指令耗盡移動力
    mud.send("wa")
    mud.send("c ref")
    
    if j.kill_action then
        send_cmds(j.kill_action)
    end
    
    -- 設定等待超時 (如果超過太久沒死，可能發生異常，嘗試強制 loot 或放棄)
    _G.ItemFarm.safe_timer(30.0, "_G.ItemFarm.wait_kill_timeout")
end

function _G.ItemFarm.wait_kill_timeout(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    if _G.ItemFarm.state.stage ~= "waiting_for_kill" then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.echo("⚠️ [" .. j.name .. "] 等待擊殺超時，嘗試強制收集戰利品...")
    _G.ItemFarm.loot(rid)
end

-- 4. 攻擊前檢查 (Score Check)
function _G.ItemFarm.do_attack(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local s = _G.ItemFarm.state
    s.stage = "checking_status_pre_fight"
    s.last_score_time = os.time()
    _G.ItemFarm.echo("📊 戰鬥前檢查狀態 (發送 score)...")
    mud.send("rep")
    mud.send("score aff")
    mud.send("save")
end

-- 直接開始戰鬥（跳過 score 檢查，用於召喚後）
function _G.ItemFarm.start_fighting(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.stage = "fighting"
    _G.ItemFarm.echo("⚔️ [" .. j.name .. "] 召喚成功，直接開始攻擊！")
    mud.emit("itemfarm_fighting", {job = j.name, mode = j.mode or "summon"})
    send_cmds(j.attack_cmd)
end

-- 根據狀態評估是否開始戰鬥
function _G.ItemFarm.evaluate_status_and_fight(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state

    -- 階段門護：防止計時器重複觸發
    if s.stage ~= "checking_status_pre_fight" then return end

    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config

    local j_hp_threshold = j.hp_threshold or cfg.hp_threshold
    local j_mp_threshold = j.mp_threshold or cfg.mp_threshold

    local hp_ok = (s.max_hp > 0) and ((j_hp_threshold == 0) or ((s.current_hp / s.max_hp) * 100 >= j_hp_threshold))
    local mp_ok = (s.max_mp > 0) and ((j_mp_threshold == 0) or ((s.current_mp / s.max_mp) * 100 >= j_mp_threshold))
    
    if not hp_ok or not mp_ok then
        local reason = not hp_ok and "HP" or "MP"
        local threshold = not hp_ok and j_hp_threshold or j_mp_threshold
        _G.ItemFarm.echo("⚠️ " .. reason .. " 不足，返回休息...「"
            .. "HP:" .. s.current_hp .. "/" .. s.max_hp 
            .. " MP:" .. s.current_mp .. "/" .. s.max_mp .. "」(" .. threshold .. "% 門檻)")
        s.stage = "returning"
        local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
        _G.ItemFarm.walk_path(path, "_G.ItemFarm.after_return")
        return
    end

    -- 智慧 Buff 檢查
    local buff_status = _G.ItemFarm.check_and_apply_buffs(s.run_id)
    if buff_status == true then
        s.stage = "fighting"
        _G.ItemFarm.echo("⚔️ [" .. j.name .. "] 狀態與 Buff 良好，開始攻擊！")
        mud.emit("itemfarm_fighting", {job = j.name, mode = j.mode or "direct"})
        send_cmds(j.attack_cmd)
    elseif buff_status == "waiting" then
        -- 等待消散中：30 秒保底檢查，其餘靠 Hook
        s.stage = "buffing" -- 防止 Ok. 觸發重複檢查
        _G.ItemFarm.safe_timer(30.0, "_G.ItemFarm.do_attack")
    else
        -- 補 Buff 中：2 秒後再次檢查
        s.stage = "buffing" -- 防止 Ok. 觸發重複檢查
        _G.ItemFarm.safe_timer(2.0, "_G.ItemFarm.do_attack")
    end
end

function _G.ItemFarm.summon_failed_too_many()
    local j = _G.ItemFarm.job()
    _G.ItemFarm.echo("⚠️ [" .. j.name .. "] 召喚失敗 3 次，跳到下一個任務...")
    _G.ItemFarm.state.summon_retries = 0
    _G.ItemFarm.state.stage = "returning"
    
    local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
    _G.ItemFarm.walk_path(path, "_G.ItemFarm.after_summon_fail")
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
    _G.ItemFarm.echo("💤 休息中...")
    mud.send(_G.ItemFarm.config.rest_cmd)
    _G.ItemFarm.safe_timer(5.0, "_G.ItemFarm.check_mp")
end

-- 5. 戰利品收集 (Looting)
function _G.ItemFarm.loot(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    
    -- [優化] 如果沒有要撿的東西也不需要 sac，直接跳到儲存階段
    if (not j.loot_items or #j.loot_items == 0) and not j.sac_corpse then
        _G.ItemFarm.echo("💨 無需撿取物品，直接前往儲存點...")
        _G.ItemFarm.go_to_storage(rid)
        return
    end
    
    _G.ItemFarm.state.stage = "looting"
    _G.ItemFarm.echo("💰 收集戰利品 (MudLoot)...")
    
    MudLoot.process_loot({
        items = j.loot_items,
        sac = j.sac_corpse,
        loot_ground = true, -- 預設開啟地面搜刮
        fallback_blind = true
    }, function()
        _G.ItemFarm.go_to_storage(_G.ItemFarm.state.run_id)
    end)
end

-- 5. 前往儲存地點
function _G.ItemFarm.go_to_storage(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    _G.ItemFarm.state.stage = "storing"
    _G.ItemFarm.echo("📦 前往儲存地點...")
    
    local path = j.path_to_storage or _G.ItemFarm.config.path_to_storage
    _G.ItemFarm.walk_path(path, "_G.ItemFarm.remove_and_drop")
end

-- 6. 整理與儲存 (Storing)
function _G.ItemFarm.remove_and_drop(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    
    local j = _G.ItemFarm.job()
    
    -- 移除 nodrop
    if j.remove_nodrop and #j.remove_nodrop > 0 then
        for _, item in ipairs(j.remove_nodrop) do
            mud.send("c 'remove n' " .. item)
        end
        _G.ItemFarm.safe_timer(1.5, "_G.ItemFarm.drop_items")
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
    _G.ItemFarm.echo("✅ [" .. j.name .. "] 收集完成 (第 " .. _G.ItemFarm.state.loot_count .. " 次)")
    mud.emit("itemfarm_loot_collected", {job = j.name, count = _G.ItemFarm.state.loot_count})
    
    _G.ItemFarm.safe_timer(2.0, "_G.ItemFarm.rest_and_repeat")
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
    
    _G.ItemFarm.echo_force("🚨 [緊急] 偵測到非預期戰鬥！嘗試逃脫並停用此任務...")
    mud.emit("itemfarm_emergency", {job = j.name})
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
    _G.ItemFarm.echo("💤 休息中...")
    mud.emit("itemfarm_resting", {job = _G.ItemFarm.job().name})
    mud.send(_G.ItemFarm.config.rest_cmd)

    _G.ItemFarm.safe_timer(5.0, "_G.ItemFarm.check_mp")
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
        mud.send("rep")
        mud.send("score aff")
        mud.send("i")
        mud.send("save")
    end
    
    _G.ItemFarm.safe_timer(5.0, "_G.ItemFarm.check_mp")
end

-- ===== Hook Registry =====
MudUtils.register_hook("ItemFarm", function(line, clean_line, is_echo)
    if is_echo then return end
    _G.ItemFarm.on_server_message(line, clean_line)
end)

-- ===== 伺服器訊息 Hook 處理器 =====
function _G.ItemFarm.on_server_message(line, clean_line)
    -- _G.ItemFarm.echo_force("[HOOK_TRACE] stage=" .. tostring(_G.ItemFarm.state.stage) .. " running=" .. tostring(_G.ItemFarm.state.running))
    if not _G.ItemFarm.state.running then return end

    local s = _G.ItemFarm.state
    -- 移除引起遞迴的直接 echo_force，改用條件式 logging 或更安全的 debug 方式
    -- if s.stage == "verifying_charm_mob" then
    --     _G.ItemFarm.echo_force("[DEBUG_VCM] clean_line: " .. clean_line)
    -- end
    
    -- MudNav/MudCombat/MudLoot 已透過 Hook Registry 自行接收訊息，無需手動委派
    
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config

    -- [優化 2: 全域環境偵測]
    if clean_line == "Ok." then
        if s.stage == "checking_status_pre_fight" or 
           s.stage == "checking_status_pre_summon" or 
           s.stage == "resting" then
            
            -- 防止洪流：如果距離上一次評估小於 0.5s，則忽略
            local now = os.clock()
            if s.last_eval_time and (now - s.last_eval_time < 0.5) then
                return
            end

            local callback = nil
            if s.stage == "checking_status_pre_fight" then
                callback = "_G.ItemFarm.evaluate_status_and_fight"
            elseif s.stage == "checking_status_pre_summon" then
                callback = "_G.ItemFarm.evaluate_status_before_summon"
            elseif s.stage == "resting" then
                callback = "_G.ItemFarm.evaluate_resting_status"
            end
            
            if callback then
                s.last_eval_time = now
                _G.ItemFarm.echo("✅ 狀態獲取完成 (Ok. 觸發)，執行評估...")
                local func = _G.ItemFarm[callback:match("ItemFarm%.(.+)") or callback]
                if func then func(s.run_id) end
            end
            return
        end
    end

    local len = #clean_line
    if len < 4 then return end 
    if string.find(clean_line, "^【") then return end 
    if string.find(clean_line, "^%s*「.*」") then return end 

    -- 體力偵測 (Move checking to MudNav, but here just check pause state if needed? MudNav handles it)
    -- MudNav will pause automatically. walk_path callback will wait.

    -- [NEW] Buff 消散偵測 (Fade Detection)
    for _, job in ipairs(_G.ItemFarm.jobs) do
        if job.buffs then
            for _, b in ipairs(job.buffs) do
                if b.fade_msg and string.find(clean_line, b.fade_msg, 1, true) then
                    _G.ItemFarm.echo("⚡ 偵測到 [" .. b.indicator .. "] 消散！更新狀態...")
                    s.active_spells[b.indicator] = nil 
                    if s.stage == "checking_status_pre_summon" or 
                       s.stage == "checking_status_pre_fight" or
                       s.stage == "resting" then
                        _G.ItemFarm.echo("🔄 偵測消散，立即重新發送檢查...")
                        _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.check_mp")
                    end
                    return
                end
            end
        end
    end

    -- 非預期戰鬥偵測 & 魅惑失敗偵測
    if s.stage ~= "fighting" and s.stage ~= "emergency" then
        if string.find(clean_line, "伺機而動", 1, true) or 
           string.find(clean_line, "蓄勢待發", 1, true) or
           string.find(clean_line, "身陷戰鬥中", 1, true) or
           string.find(clean_line, "不受你的言語所迷惑", 1, true) then
            
            if s.stage == "charming" or s.stage == "fleeing_for_charm" then
                if s.stage == "charming" then
                    _G.ItemFarm.handle_charm_failed(j, "combat")
                elseif s.stage == "fleeing_for_charm" then
                    mud.send("fl") -- 持續逃跑
                end
                return
            else
                _G.ItemFarm.emergency_escape()
            end
            return
        end
    end

    -- [優化 3: 階段精確分流]
    if s.stage == "fighting" then
        -- 戰鬥階段
        if string.find(clean_line, "魂歸西天了", 1, true) and match_target(clean_line, j.target_mob) then
            _G.ItemFarm.echo("💀 目標已擊殺！")
            mud.emit("itemfarm_target_killed", {job = j.name})
            _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.loot")
        elseif match_target(clean_line, j.target_mob) and 
               (string.find(clean_line, "逃了", 1, true) or string.find(clean_line, "離開了", 1, true)) then
            _G.ItemFarm.handle_mob_fled(j)
        elseif string.find(clean_line, "目標不在", 1, true) or string.find(clean_line, "施法的目標不在", 1, true) then
            _G.ItemFarm.handle_mob_missing(j)
        end
        return

    elseif s.stage == "waiting_for_kill" then
        -- 魅惑模式下等待目標死亡或掉落物品
        if string.find(clean_line, "丟下了", 1, true) or 
           (string.find(clean_line, "魂歸西天了", 1, true) and match_target(clean_line, j.target_mob)) then
            _G.ItemFarm.echo("💀 目標已死亡或掉落物品！準備撿取...")
            _G.ItemFarm.safe_timer(1.0, "_G.ItemFarm.loot")
        end
        return
        
    elseif s.stage == "charming" then
        -- 判定魅惑成功
        if string.find(clean_line, "開始跟隨你了", 1, true) then
            _G.ItemFarm.echo("💖 魅惑成功！")
            s.charm_retries = 0
            _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.lead_to_kill_zone")
        elseif string.find(clean_line, "沒有這個生物", 1, true) then
            _G.ItemFarm.handle_mob_missing(j)
        end
        return

    elseif s.stage == "summoning" then
        -- 召喚階段由 MudCombat 接管 (Success/Fail/Retry)
        -- 這裡只需要等待 callback 觸發
        return

    elseif s.stage == "searching" then
        -- 搜尋階段
        local found = false
        if j.search_type == "quest" then
            if string.find(clean_line, "他正在這個世界中", 1, true) then found = true end
        elseif j.search_type == "locate" then
            if string.find(clean_line, "攜帶著", 1, true) and match_target(clean_line, j.target_mob) then
                found = true
            end
        end
        if found then
            _G.ItemFarm.echo("🎯 [" .. j.name .. "] 目標存在！前往戰鬥...")
            s.found_target = true
            s.stage = "traveling"
            _G.ItemFarm.safe_timer(1.0, "_G.ItemFarm.go_and_fight")
        end
        return

    elseif s.stage == "verifying_mob" or s.stage == "verifying_charm_mob" then
        -- 驗證存在
        -- [修正] 忽略房間 ID 行，避免誤判
        if string.find(clean_line, "^(ID: ", 1, true) then return end
        
        local match_target = false
        if type(j.target_mob) == "table" then
            match_target = false
            for _, kw in ipairs(j.target_mob) do
                if string.find(clean_line, kw, 1, true) then
                    match_target = true
                    break
                end
            end
        else
            match_target = string.find(clean_line, j.target_mob, 1, true)
        end

        if match_target and
           not string.find(clean_line, "屍體", 1, true) and
           not string.find(clean_line, "corpse", 1, true) then
            _G.ItemFarm.echo("✅ 目標在場！")
            
            if s.stage == "verifying_mob" then
                s.stage = "verified"
                _G.ItemFarm.start_dispel_or_attack(s.run_id)
            elseif s.stage == "verifying_charm_mob" then
                s.stage = "verified"
                _G.ItemFarm.do_charm_and_lead(s.run_id)
            end
        end
        return

    elseif s.stage == "checking_dispel" then
        -- 檢查 Dispel
        local match_target = false
        if type(j.target_mob) == "table" then
            match_target = false
            for _, kw in ipairs(j.target_mob) do
                if string.find(clean_line, kw, 1, true) then
                    match_target = true
                    break
                end
            end
        else
            match_target = string.find(clean_line, j.target_mob, 1, true)
        end

        if match_target and
           not string.find(clean_line, "屍體", 1, true) and
           not string.find(clean_line, "corpse", 1, true) then
            local active_indicator = nil
            if j.dispel_indicators then
                for _, ind in ipairs(j.dispel_indicators) do
                    if string.find(clean_line, ind, 1, true) then
                        active_indicator = ind
                        break
                    end
                end
            end
            
            if active_indicator then
                _G.ItemFarm.echo("❌ 偵測到防護：" .. active_indicator .. "，重試 Dispel...")
                s.stage = "dispelling"
                _G.ItemFarm.retry_dispel_with_look(s.run_id)
            else
                _G.ItemFarm.echo("✅ Dispel 成功！目標無殘餘防護")
                s.dispel_retries = 0
                s.stage = "dispelled"
                _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.buff_and_attack")
            end
        end
        return

    elseif s.stage == "checking_status_pre_fight" or 
           s.stage == "checking_status_pre_summon" or 
           s.stage == "resting" then
        
        -- [優化：支援 rep 快速報數解析]
        -- 你報告自己的狀況: 2151/2151 生命力 5964/5964 精神力 394/584 移動力 165/165 內力
        if string.find(clean_line, "你報告自己的狀況", 1, true) then
            local h_cur, h_max = string.match(clean_line, "(%d+)/(%d+)%s+生命力")
            local m_cur, m_max = string.match(clean_line, "(%d+)/(%d+)%s+精神力")
            if h_cur and h_max then
                s.current_hp = tonumber(h_cur)
                s.max_hp = tonumber(h_max)
            end
            if m_cur and m_max then
                s.current_mp = tonumber(m_cur)
                s.max_mp = tonumber(m_max)
            end

            -- rep 是單行訊息，直接觸發評估
            local callback = "evaluate_status_and_fight"
            if s.stage == "checking_status_pre_summon" then
                callback = "evaluate_status_before_summon"
            elseif s.stage == "resting" then
                callback = "evaluate_resting_status"
            end
            -- rep 增加保底計時器至 3.0s，確保隨後的 score aff 有足夠時間解析
            -- 實際觸發將優先由隨後的 "Ok." (來自 save 指令) 截斷執行
            _G.ItemFarm.safe_timer(3.0, "_G.ItemFarm." .. callback)
            return
        end

        -- Score 解析區塊 (支援冒號可選，單行多欄位)
        local h_cur, h_max = string.match(clean_line, "生命力:?%s+(%d+)/%s+(%d+)")
        if h_cur and h_max then
            s.current_hp = tonumber(h_cur)
            s.max_hp = tonumber(h_max)
        end
        
        local m_cur, m_max = string.match(clean_line, "精神力:?%s+(%d+)/%s+(%d+)")
        if m_cur and m_max then
            s.current_mp = tonumber(m_cur)
            s.max_mp = tonumber(m_max)
        end

        -- 如果是生命力那一張圖，解析完就 return 避免後續重複判斷
        if h_cur then return end

        local spell_name, hours = string.match(clean_line, "法術:%s+'(.-)'.*達%s+(-?%d+)%s+小時")
        if spell_name then
            s.active_spells[spell_name] = tonumber(hours)
            return
        end

        if string.find(clean_line, "目前對你產生影響的法術或技巧有", 1, true) then
            s.active_spells = {}
            return
        end

        -- 使用「空行」或「提示符」作為 score 結束的判定點並不保險
        -- 我們維持原有的 timer 延遲觸發，但加入對 score 顯示內容的過濾
        if string.find(clean_line, "行動力", 1, true) or string.find(clean_line, "內力指數", 1, true) then
            -- 這是 score sit 的最後一行內容，可以觸發評估
            local callback = "evaluate_status_and_fight"
            if s.stage == "checking_status_pre_summon" then
                callback = "evaluate_status_before_summon"
            elseif s.stage == "resting" then
                callback = "evaluate_resting_status"
            end
            _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm." .. callback)
            return
        end
    end

    -- 剩餘少數特殊狀態
    if s.stage == "emergency" or s.stage == "fleeing_for_charm" then
        if string.find(clean_line, "你為了保命而不顧面子從戰鬥中逃了", 1, true) or
           string.find(clean_line, " recall", 1, true) then
            _G.ItemFarm.echo("✅ 成功逃離戰鬥！")
            
            if s.stage == "fleeing_for_charm" then
                s.stage = "retry_charming"
                _G.ItemFarm.echo("🔄 準備重試魅惑，導航回目標點...")
                -- 改用 go_and_fight 確保導航回正確位置
                _G.ItemFarm.safe_timer(2.0, "_G.ItemFarm.go_and_fight")
            else
                s.stage = "idle"
                _G.ItemFarm.next_job()
            end
        elseif string.find(clean_line, "你逃跑失敗了", 1, true) then
            mud.send("fl")
        end
        return
    end

    if s.stage == "verifying_loc" then
        if string.find(clean_line, "攜帶著", 1, true) then
            _G.ItemFarm.echo("🚫 [" .. j.name .. "] 目標在別處！永久停用此任務（需手動找回）")
            j.disabled = true
            s.stage = "returning"
            _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return")
        end
        return
    end
end


-- ===== 輔助邏輯與回呼函數 =====

-- 處理目標逃跑 (Fled)
function _G.ItemFarm.handle_mob_fled(j)
    local s = _G.ItemFarm.state
    local mode = j.mode or "summon"
    if mode == "summon" then
        _G.ItemFarm.echo("🏃 目標逃跑了！重新召喚...")
        s.stage = "summoning"
        s.summon_retries = 0
        _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.summon_and_attack")
    else
        _G.ItemFarm.echo("🏃 目標逃跑了！返回儲存點...")
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return")
    end
end

-- 處理目標消失 (Missing)
function _G.ItemFarm.handle_mob_missing(j)
    local s = _G.ItemFarm.state
    local mode = j.mode or "summon"
    if mode == "summon" then
        _G.ItemFarm.echo("❌ 目標不在這裡！重新召喚...")
        s.stage = "summoning"
        s.summon_retries = 0
        _G.ItemFarm.safe_timer(0.5, "_G.ItemFarm.summon_and_attack")
    else
        _G.ItemFarm.echo("❌ 目標不在這裡！返回儲存點...")
        s.stage = "returning"
        _G.ItemFarm.walk_path(j.path_to_storage, "_G.ItemFarm.after_return")
    end
end

-- 評估休息階段的狀態 (起身/續睡/補聖光)
function _G.ItemFarm.evaluate_resting_status(rid)
    if not check_run(rid) then return end
    if not _G.ItemFarm.state.running then return end
    local s = _G.ItemFarm.state
    
    -- 階段門護：防止計時器重複觸發
    if s.stage ~= "resting" then return end
    
    local j = _G.ItemFarm.job()
    local cfg = _G.ItemFarm.config
    
    -- 在休息階段檢查是否可以起床
    local hp_pct = (s.max_hp > 0) and math.floor((s.current_hp / s.max_hp) * 100) or 100
    local mp_pct = (s.max_mp > 0) and math.floor((s.current_mp / s.max_mp) * 100) or 100
    
    local j_hp_threshold = j.hp_threshold or cfg.hp_threshold
    local j_mp_threshold = j.mp_threshold or cfg.mp_threshold
    
    local hp_ok = (s.max_hp > 0) and ((j_hp_threshold == 0) or ((s.current_hp / s.max_hp) * 100 >= j_hp_threshold))
    local mp_ok = (s.max_mp > 0) and ((j_mp_threshold == 0) or ((s.current_mp / s.max_mp) * 100 >= j_mp_threshold))
    
    -- 如果 HP 不足且有恢復指令
    if not hp_ok and j.hp_recover_cmd then
        _G.ItemFarm.echo("⚡ HP 不足，站立並執行恢復: " .. j.hp_recover_cmd)
        mud.send("wa")
        mud.send(j.hp_recover_cmd)
        mud.send(cfg.rest_cmd)
        return
    end

    -- 智慧 Buff 維持 (休息中也要補)
    if hp_ok and mp_ok then
        local b = _G.ItemFarm.get_missing_buff(s.run_id)
        if b then
            _G.ItemFarm.echo("✨ 補 Buff (休息中): " .. b.indicator .. " (" .. b.cmd .. ")")
            -- 順序：站立 → 施法 → 繼續休息
            mud.send("wa")
            mud.send(b.cmd)
            mud.send(cfg.rest_cmd)
            return
        end
    end

    if hp_ok and mp_ok then
        _G.ItemFarm.echo("✅ 狀態已回滿且 Buff 齊全 (HP:" .. s.current_hp .. " MP:" .. s.current_mp .. ")，切換至下一任務...")
        s.stage = "idle"
        mud.send("wa")
        _G.ItemFarm.next_job()
    end
end

function _G.ItemFarm.reload()
    package.loaded["scripts.itemfarm"] = nil
    require("scripts.itemfarm")
    _G.ItemFarm.echo("♻️ 腳本已重新載入")
end

-- ===== 初始化腳本 =====
_G.ItemFarm.init()

mud.echo("ItemFarm script loaded successfully.")

return _G.ItemFarm
