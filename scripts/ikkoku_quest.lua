-- ============================================================
-- IkkokuQuest - 相聚一刻解謎任務自動腳本
-- ============================================================
-- 使用: /lua IkkokuQuest.start()
-- 停止: /lua IkkokuQuest.stop()
-- 狀態: /lua IkkokuQuest.status()
-- ============================================================

_G.IkkokuQuest = _G.IkkokuQuest or {}

local string = string
local table = table
local ipairs = ipairs
local tonumber = tonumber
local math = math

-- ===== 方向映射 =====
local DIR_INFO = {
    {name="北", cmd="n", dx=0, dy=1, dz=0},
    {name="南", cmd="s", dx=0, dy=-1, dz=0},
    {name="東", cmd="e", dx=1, dy=0, dz=0},
    {name="西", cmd="w", dx=-1, dy=0, dz=0},
    {name="上", cmd="u", dx=0, dy=0, dz=1},
    {name="下", cmd="d", dx=0, dy=0, dz=-1},
}

local DIR_BY_NAME = {}
local DIR_BY_CMD = {}
for _, d in ipairs(DIR_INFO) do
    DIR_BY_NAME[d.name] = d
    DIR_BY_CMD[d.cmd] = d
end

local REVERSE_CMD = {n="s", s="n", e="w", w="e", u="d", d="u"}
local DIR_PRIORITY = {"北", "東", "南", "西", "上", "下"}

local function pos_key(pos)
    return pos.x .. "," .. pos.y .. "," .. pos.z
end

local function parse_exits(line)
    local exits = {}
    local exit_str = string.match(line, "%[出口:%s*(.-)%]")
    if exit_str then
        for dir in string.gmatch(exit_str, "%S+") do
            if DIR_BY_NAME[dir] then
                exits[#exits + 1] = dir
            end
        end
    end
    return exits
end

-- ===== 設定 =====
_G.IkkokuQuest.config = {
    entry_path = "6w;3n;enter ikkoku",
    max_find_laps = 5,
    -- 已知房間路徑 (從 enter ikkoku 後)
    path_to_room3 = "n;open n;n;2e;n;u;s;w;open n;n",   -- 三號房 (akemi)
    path_to_room4 = "n;open n;n;2e;n;u;s;2w;open n;n",  -- 四號房 (godai)
    path_to_room5 = "n;open n;n;2e;n;u;s;3w;open n;n",  -- 五號房 (yotsuya)
    path_to_manager = "2e;3n;w;op s;s",         -- 管理人室 (kyokoo/yukari)
    -- recall 後
    path_to_keeper_area = "6w;3n;n;w",          -- 茶茶丸酒吧外 (keeper)
    path_to_entrance = "2e;3n;w;op s;s;s;e",    -- 玄關 (otonashi sum 點)

    default_door_dirs = {"n", "s", "e", "w"},   -- 預設只走東西南北，Ikkoku 沒上下樓的門
}

-- ===== 任務步驟定義 =====
-- 每步: target=要找的mob, cmds=找到後發的指令, expect=成功判定關鍵字, next=下一步
local QUEST_STEPS = {
    {name="wait_kyokoo",    target="kyokoo",   cmds={"talk kyokoo otonashi", "talk kyokoo yes"}, expect="看能不能說服他進來", next="find_otonashi_1"},
    {name="find_otonashi_1", target="otonashi", cmds={"talk otonashi kyokoo"}, expect="不要....叫響子出來見我..!!", next="find_kyokoo_2"},
    {name="find_kyokoo_2",   target="kyokoo",   cmds={"talk kyokoo otonashi"}, expect="也許五代有辦法，你去問他看看吧...", next="find_godai_1"},
    {name="find_godai_1",    target="godai",    cmds={"talk godai otonashi"}, expect="也許我奶奶有辦法吧....你去問看看吧..", next="find_yukari"},
    {name="find_yukari",     target="yukari",   cmds={"talk yukari godai", "talk yukari otonashi"}, expect="五代由加莉 把 錦囊 給了你.", next="find_godai_2"},
    {name="find_godai_2",    target="godai",    cmds={"gi bag godai"}, expect="我奶奶說可以試著找四谷先生幫忙...不過四谷是個很怪的人喔..", next="find_yotsuya"},
    {name="find_yotsuya",    target="yotsuya",  cmds={"talk yotsuya godai"}, expect="我想找朱美比較好解決吧.", next="find_akemi_1"},
    {name="find_akemi_1",    target="akemi",    cmds={"talk akemi yotsuya"}, expect="那麼你只要給我一瓶茶茶丸的白酒", next="go_keeper"},
    {name="go_keeper",       target="keeper",   cmds={"talk keeper akemi"}, expect="好...你跟我來一下...", next="chachamaru"},
    {name="chachamaru",      target="keeper",   cmds={"talk keeper akemi"}, expect="茶茶丸的老闆 把 白酒 給了你", next="find_akemi_2"},
    {name="find_akemi_2",    target="akemi",    cmds={"gi wine akemi"}, expect="你把 白酒 給了 朱美.", next="find_otonashi_2"},
    {name="find_otonashi_2", target="otonashi", cmds={"talk otonashi kyokoo"}, expect="", next="done"},
}

local STEP_BY_NAME = {}
for i, step in ipairs(QUEST_STEPS) do
    STEP_BY_NAME[step.name] = i
end

-- ===== 指令解析 =====
local function parse_cmds(str)
    local result = {}
    for cmd in string.gmatch(str, "[^;]+") do
        cmd = cmd:match("^%s*(.-)%s*$")
        if cmd ~= "" then
            local count, actual = cmd:match("^(%d+)(%a.*)$")
            if count then
                for _ = 1, tonumber(count) do
                    result[#result + 1] = actual
                end
            else
                result[#result + 1] = cmd
            end
        end
    end
    return result
end

-- ===== 狀態 =====
_G.IkkokuQuest.state = {
    running = false,
    run_id = 0,
    phase = "idle",
    step_index = 0,
    -- 行走
    path_queue = {},
    path_index = 0,
    path_callback = nil,
    walking = false,
    path_paused = false,
    walk_expected = false,
    -- DFS 探索器
    explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        last_exit_line = nil,
        room_count = 0,
        laps = 0,
        doors_opened = false,
    },
    -- mob 偵測
    target_in_room = false,
    target_line = nil,
    -- 房間名稱偵測
    room_found = false,
    -- 等待 mob
    wait_timer_active = false,
}

-- ===== run_id 檢查 =====
local function check_run(rid)
    if not rid then return true end
    return rid == _G.IkkokuQuest.state.run_id
end

-- ===== 訊息 =====
function _G.IkkokuQuest.echo(msg)
    mud.echo("[IkkokuQuest] " .. msg)
end

-- ===== Timer =====
-- ===== Timer =====
_G.IkkokuQuest.callbacks = {}
_G.IkkokuQuest.callback_id = 0

function _G.IkkokuQuest.safe_timer(seconds, func_or_name)
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    if type(func_or_name) == "function" then
        _G.IkkokuQuest.callback_id = _G.IkkokuQuest.callback_id + 1
        local cb_id = _G.IkkokuQuest.callback_id
        _G.IkkokuQuest.callbacks[cb_id] = func_or_name
        
        -- 透過 exec_callback 執行
        local code = "_G.IkkokuQuest.exec_callback(" .. cb_id .. ", " .. s.run_id .. ")"
        mud.timer(seconds, code)
    else
        -- 舊模式：字串函數名
        local code = func_or_name .. "(" .. s.run_id .. ")"
        mud.timer(seconds, code)
    end
end

function _G.IkkokuQuest.exec_callback(cb_id, rid)
    local func = _G.IkkokuQuest.callbacks[cb_id]
    if func then
        func(rid)
        _G.IkkokuQuest.callbacks[cb_id] = nil -- 執行後清除
    end
end

-- ============================================================
-- 行走系統 (walk_path)
-- ============================================================
function _G.IkkokuQuest.walk_path(str, callback)
    local s = _G.IkkokuQuest.state
    s.path_queue = parse_cmds(str)
    s.path_index = 1
    s.path_callback = callback
    s.path_paused = false
    s.walking = true
    _G.IkkokuQuest.walk_send(s.run_id)
end

function _G.IkkokuQuest.walk_send(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    if s.path_index > #s.path_queue then
        s.walking = false
        s.walk_expected = false
        s.path_queue = {}
        s.path_index = 0
        if s.path_callback then
            _G.IkkokuQuest.safe_timer(0.5, s.path_callback)
        end
        return
    end

    local cmd = s.path_queue[s.path_index]
    -- 判斷是否為移動指令 (單字母方向)
    local is_move = DIR_BY_CMD[cmd] ~= nil
    if is_move then
        s.walk_expected = true
    else
        s.walk_expected = false
    end
    mud.send(cmd)
    -- 非移動指令 → 不等 [出口:]，直接延遲推進
    if not is_move then
        _G.IkkokuQuest.safe_timer(0.5, "_G.IkkokuQuest.walk_advance_timer")
    end
end

function _G.IkkokuQuest.walk_advance()
    local s = _G.IkkokuQuest.state
    s.walk_expected = false
    s.path_index = s.path_index + 1
    _G.IkkokuQuest.safe_timer(0.05, "_G.IkkokuQuest.walk_send")
end

-- timer 版本 (帶 rid 參數)
function _G.IkkokuQuest.walk_advance_timer(rid)
    if not check_run(rid) then return end
    _G.IkkokuQuest.walk_advance()
end

function _G.IkkokuQuest.recover_stamina(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    s.path_paused = true
    _G.IkkokuQuest.echo("✨ 施放 refresh 並等待恢復...")
    mud.send("c ref")
end

function _G.IkkokuQuest.walk_resume(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    s.path_paused = false
    _G.IkkokuQuest.walk_send(s.run_id)
end

-- ============================================================
-- recall_and_go: recall → enter ikkoku → 走指定路徑 → callback
-- ============================================================
function _G.IkkokuQuest.recall_and_go(path_from_ikkoku, callback)
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    mud.send("recall")
    -- 組合: recall後 → entry_path → 房間路徑
    local full_path = _G.IkkokuQuest.config.entry_path .. ";" .. path_from_ikkoku
    s.recall_callback = callback
    s.recall_path = full_path
    _G.IkkokuQuest.safe_timer(1.5, "_G.IkkokuQuest.recall_then_walk")
end

function _G.IkkokuQuest.recall_then_walk(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    _G.IkkokuQuest.walk_path(s.recall_path, s.recall_callback)
end

-- ============================================================
-- 通用等待 mob 機制 (到達指定位置後每 5 秒 look)
-- ============================================================
function _G.IkkokuQuest.wait_mob_start(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    s.phase = "waiting"
    _G.IkkokuQuest.echo("⏳ 等待 " .. (step and step.target or "?") .. " 出現 (每 5 秒 look)...")
    _G.IkkokuQuest.wait_mob_check(s.run_id)
end

function _G.IkkokuQuest.wait_mob_check(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    if s.phase ~= "waiting" or s.path_paused then return end

    s.target_in_room = false
    mud.send("l")
    _G.IkkokuQuest.safe_timer(5.0, "_G.IkkokuQuest.wait_mob_retry")
end

function _G.IkkokuQuest.wait_mob_retry(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    if s.phase ~= "waiting" then return end
    
    local step = QUEST_STEPS[s.step_index]
    if step and step.target == "yotsuya" then
        -- 四谷三點巡迴: 五號房 <-> 牆縫 <-> 四號房
        if s.yotsuya_pos == "room5" then
            _G.IkkokuQuest.echo("🚶 鑽進牆縫尋找四谷...")
            mud.send("squeeze")
            s.yotsuya_dir = "east"
        elseif s.yotsuya_pos == "gap" then
            if s.yotsuya_dir == "east" then
                _G.IkkokuQuest.echo("🚶 鑽向四號房尋找四谷...")
                mud.send("squeeze east")
            else
                _G.IkkokuQuest.echo("🚶 鑽向五號房尋找四谷...")
                mud.send("squeeze west")
            end
        elseif s.yotsuya_pos == "room4" then
            _G.IkkokuQuest.echo("🚶 鑽回牆縫尋找四谷...")
            mud.send("squeeze")
            s.yotsuya_dir = "west"
        else
            -- 位置未知，重回五號房
            _G.IkkokuQuest.echo("⚠️ 位置未知，重回五號房搜尋...")
            mud.send("recall")
            _G.IkkokuQuest.walk_path(_G.IkkokuQuest.config.path_to_room5, "_G.IkkokuQuest.wait_mob_start")
            return
        end
    end
    
    _G.IkkokuQuest.wait_mob_check(s.run_id)
end

-- ============================================================
-- DFS 探索系統
-- ============================================================

function _G.IkkokuQuest.start_find(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    if not step or not step.target then return end

    _G.IkkokuQuest.echo("🔍 開始搜尋: " .. step.target)
    s.phase = "exploring"
    s.target_in_room = false
    s.target_line = nil
    -- 重置探索器
    s.explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        last_exit_line = nil,
        room_count = 0,
        laps = 0,
        doors_opened = false,
    }
    -- 開門 + look
    local door_dirs = step.door_dirs or _G.IkkokuQuest.config.default_door_dirs
    if door_dirs then
        for _, dir in ipairs(door_dirs) do
            mud.send("op " .. dir)
        end
    end
    mud.send("l")
end

function _G.IkkokuQuest.explore_room_dispatch(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    if s.phase ~= "exploring" then return end

    local exp = s.explorer
    -- 前進到新房間: 先開門再重新 look
    if exp.pending and exp.pending.type == "forward" and not exp.doors_opened then
        exp.doors_opened = true
        local step = QUEST_STEPS[s.step_index]
        local door_dirs = step.door_dirs or _G.IkkokuQuest.config.default_door_dirs
        if door_dirs then
            for _, dir in ipairs(door_dirs) do
                mud.send("op " .. dir)
            end
        end
        -- 不重置 target_in_room，讓 mob 偵測跨兩次 look 累積
        mud.send("l")
        return
    end

    exp.doors_opened = false
    _G.IkkokuQuest.explore_room(rid, s.explorer.last_exit_line or "")
end

function _G.IkkokuQuest.explore_room(rid, exit_line)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local exp = s.explorer

    -- 更新座標
    if exp.pending then
        if exp.pending.type == "forward" then
            local d = exp.pending.d
            exp.pos = {x=exp.pos.x+d.dx, y=exp.pos.y+d.dy, z=exp.pos.z+d.dz}
            exp.path[#exp.path + 1] = {cmd=d.cmd, rev=REVERSE_CMD[d.cmd]}
        elseif exp.pending.type == "backtrack" then
            local rev = exp.pending.rev_cmd
            local d_back = DIR_BY_CMD[rev]
            if d_back then
                exp.pos = {x=exp.pos.x+d_back.dx, y=exp.pos.y+d_back.dy, z=exp.pos.z+d_back.dz}
            end
            if #exp.path > 0 then table.remove(exp.path) end
        end
        exp.pending = nil
    end

    exp.exits = parse_exits(exit_line)

    local key = pos_key(exp.pos)
    if not exp.visited[key] then
        exp.visited[key] = true
        exp.room_count = exp.room_count + 1
    end

    -- 找到目標 mob 或房間
    local step = QUEST_STEPS[s.step_index]

    -- wait_kyokoo 特殊: 找到管理人室後等待 Kyokoo
    if step and step.name == "wait_kyokoo" and s.room_found then
        if s.target_in_room then
            -- Kyokoo 已在場！直接執行
            _G.IkkokuQuest.echo("🏠 到達管理人室，Kyokoo 已在場！")
            _G.IkkokuQuest.execute_step_cmds(s.run_id)
        else
            -- 到達管理人室，等待 Kyokoo
            _G.IkkokuQuest.echo("🏠 到達管理人室！")
            _G.IkkokuQuest.wait_kyokoo_start(s.run_id)
        end
        return
    end

    if s.target_in_room then
        local step = QUEST_STEPS[s.step_index]
        _G.IkkokuQuest.echo("🎯 找到 " .. step.target .. "！")
        if s.target_line then
            _G.IkkokuQuest.echo("   " .. s.target_line)
        end
        -- 執行該步驟的指令
        _G.IkkokuQuest.execute_step_cmds(s.run_id)
        return
    end

    _G.IkkokuQuest.explore_next(s.run_id)
end

function _G.IkkokuQuest.explore_next(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    s.phase = "exploring"
    local exp = s.explorer

    for _, dir_name in ipairs(DIR_PRIORITY) do
        local has_exit = false
        for _, ex in ipairs(exp.exits) do
            if ex == dir_name then has_exit = true; break end
        end
        if has_exit then
            local d = DIR_BY_NAME[dir_name]
            local next_key = pos_key({x=exp.pos.x+d.dx, y=exp.pos.y+d.dy, z=exp.pos.z+d.dz})
            if not exp.visited[next_key] then
                exp.pending = {type="forward", d=d}
                s.target_in_room = false
                s.explorer.last_exit_line = nil
                mud.send(d.cmd)
                return
            end
        end
    end

    -- 回溯
    if #exp.path > 0 then
        local last = exp.path[#exp.path]
        exp.pending = {type="backtrack", rev_cmd=last.rev}
        s.target_in_room = false
        s.explorer.last_exit_line = nil
        mud.send(last.rev)
    else
        -- 一圈完畢
        exp.laps = (exp.laps or 0) + 1
        local max_laps = _G.IkkokuQuest.config.max_find_laps or 5
        _G.IkkokuQuest.echo("🔄 第 " .. exp.laps .. " 圈完畢 (共 " .. exp.room_count .. " 間)")

        if exp.laps >= max_laps then
            local step = QUEST_STEPS[s.step_index]
            _G.IkkokuQuest.echo("❌ 已探索 " .. max_laps .. " 圈，未找到: " .. (step and step.target or "?"))
            _G.IkkokuQuest.stop()
        else
            _G.IkkokuQuest.echo("🔍 Mob 可能已移動，開始第 " .. (exp.laps + 1) .. " 圈...")
            exp.visited = {}
            exp.visited[pos_key(exp.pos)] = true
            s.target_in_room = false
            _G.IkkokuQuest.explore_next(s.run_id)
        end
    end
end

function _G.IkkokuQuest.retry_move(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    local exp = s.explorer
    if exp.pending then
        local cmd = exp.pending.type == "forward" and exp.pending.d.cmd or exp.pending.rev_cmd
        s.target_in_room = false
        s.explorer.last_exit_line = nil
        mud.send(cmd)
    end
end

-- ============================================================
-- 步驟執行
-- ============================================================

-- 找到 mob 後執行該步驟的指令
function _G.IkkokuQuest.execute_step_cmds(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    if not step then return end

    s.phase = "acting"
    for _, cmd in ipairs(step.cmds) do
        mud.send(cmd)
    end

    -- 等待回應後推進到下一步
    _G.IkkokuQuest.safe_timer(3.0, "_G.IkkokuQuest.advance_step")
end

-- 推進到下一步
function _G.IkkokuQuest.advance_step(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    if not step then _G.IkkokuQuest.stop(); return end

    local next_name = step.next
    if next_name == "done" then
        _G.IkkokuQuest.quest_complete(s.run_id)
        return
    end

    local next_idx = STEP_BY_NAME[next_name]
    if not next_idx then
        _G.IkkokuQuest.echo("⚠️ 找不到步驟: " .. next_name)
        _G.IkkokuQuest.stop()
        return
    end

    s.step_index = next_idx
    local next_step = QUEST_STEPS[next_idx]
    _G.IkkokuQuest.echo("📋 進入步驟: " .. next_step.name)

    _G.IkkokuQuest.run_step(s.run_id)
end

-- 執行當前步驟
function _G.IkkokuQuest.run_step(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    if not step then return end

    s.step_completed = false -- 重置步驟完成標記

    -- === 特殊步驟處理 ===

    -- wait_kyokoo: 直接走到管理人室等待 Kyokoo
    if step.name == "wait_kyokoo" then
        _G.IkkokuQuest.echo("🏠 前往管理人室...")
        _G.IkkokuQuest.walk_path(_G.IkkokuQuest.config.path_to_manager, "_G.IkkokuQuest.wait_mob_start")
        return
    end

    -- find_yukari / find_kyokoo_2: recall → 管理人室 (固定位置)
    if step.name == "find_yukari" then
        _G.IkkokuQuest.echo("🏠 前往管理人室找 " .. step.target .. "...")
        _G.IkkokuQuest.recall_and_go(_G.IkkokuQuest.config.path_to_manager, "_G.IkkokuQuest.wait_mob_start")
        return
    end

    if step.name == "find_kyokoo_2" then
        _G.IkkokuQuest.echo("🏠 從玄關走回管理人室找 Kyokoo...")
        _G.IkkokuQuest.walk_path("w;n", "_G.IkkokuQuest.wait_mob_start")
        return
    end

    -- find_akemi_1 / find_akemi_2: recall → 三號房等待朱美
    if step.name == "find_akemi_1" or step.name == "find_akemi_2" then
        _G.IkkokuQuest.echo("🏠 前往三號房找朱美 Akemi...")
        _G.IkkokuQuest.recall_and_go(_G.IkkokuQuest.config.path_to_room3, "_G.IkkokuQuest.wait_mob_start")
        return
    end

    -- find_godai: recall → 四號房等待
    if step.name == "find_godai_1" or step.name == "find_godai_2" then
        _G.IkkokuQuest.echo("🏠 前往四號房找 " .. step.target .. "...")
        _G.IkkokuQuest.recall_and_go(_G.IkkokuQuest.config.path_to_room4, "_G.IkkokuQuest.wait_mob_start")
        return
    end

    -- find_yotsuya: recall → 五號房等待
    if step.name == "find_yotsuya" then
        _G.IkkokuQuest.echo("🏠 前往五號房找四谷 Yotsuya...")
        _G.IkkokuQuest.recall_and_go(_G.IkkokuQuest.config.path_to_room5, "_G.IkkokuQuest.wait_mob_start")
        return
    end

    -- go_keeper: 去酒吧外面等老闆
    if step.name == "go_keeper" then
        _G.IkkokuQuest.echo("🏠 前往酒吧外找老闆 keeper...")
        mud.send("recall")
        _G.IkkokuQuest.walk_path(_G.IkkokuQuest.config.path_to_keeper_area, "_G.IkkokuQuest.wait_mob_start")
        return
    end

    -- chachamaru: 進入酒吧等老闆
    if step.name == "chachamaru" then
        _G.IkkokuQuest.echo("🏠 進入酒吧 chachamaru 找 keeper...")
        mud.send("enter chachamaru")
        _G.IkkokuQuest.wait_mob_start(s.run_id)
        return
    end

    -- find_otonashi: 優先召喚 (必須在玄關)
    if step.name == "find_otonashi_1" then
        -- 從管理人室出發 -> s;e -> 玄關
        _G.IkkokuQuest.echo("✨ 前往玄關召喚音無爸爸...")
        _G.IkkokuQuest.walk_path("open s;s;e", "_G.IkkokuQuest.do_summon_otonashi")
        return
    end

    if step.name == "find_otonashi_2" then
        -- 從任意點 recalls -> 玄關
        _G.IkkokuQuest.echo("✨ 前往玄關召喚音無爸爸...")
        _G.IkkokuQuest.recall_and_go(_G.IkkokuQuest.config.path_to_entrance, "_G.IkkokuQuest.do_summon_otonashi")
        return
    end

    -- === 一般步驟: DFS 找 mob ===
    if step.target then
        _G.IkkokuQuest.start_find(s.run_id)
        return
    end

    -- 無 target 無特殊處理 → 直接執行指令
    _G.IkkokuQuest.execute_step_cmds(s.run_id)
end

-- 任務完成
function _G.IkkokuQuest.quest_complete(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    s.wait_timer_active = false
    s.yotsuya_pos = "room5"
    s.yotsuya_dir = "east"
    s.running = false
    s.phase = "done"
    _G.IkkokuQuest.echo("═══════════════════════════════════════")
    _G.IkkokuQuest.echo("🎉 相聚一刻任務完成！")
    _G.IkkokuQuest.echo("═══════════════════════════════════════")
end

-- 召喚音無爸爸
function _G.IkkokuQuest.do_summon_otonashi(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    
    _G.IkkokuQuest.echo("✨ 施放 summon otonashi...")
    mud.send("c sum otonashi")
    _G.IkkokuQuest.wait_mob_start(s.run_id)
end

-- ============================================================
-- Server Hook
-- ============================================================

local base_hook = nil
if _G.on_server_message and not _G.IkkokuQuest.hook_installed then
    base_hook = _G.on_server_message
elseif _G.IkkokuQuest._base_hook then
    base_hook = _G.IkkokuQuest._base_hook
end
_G.IkkokuQuest._base_hook = base_hook

_G.on_server_message = function(line, clean_line)
    if base_hook then base_hook(line, clean_line) end
    if _G.IkkokuQuest and _G.IkkokuQuest.on_server_message then
        _G.IkkokuQuest.on_server_message(line, clean_line)
    end
end
_G.IkkokuQuest.hook_installed = true

function _G.IkkokuQuest.on_server_message(line, clean_line)
    if not _G.IkkokuQuest.state.running then return end
    local s = _G.IkkokuQuest.state
    if not clean_line or #clean_line < 3 then return end

    -- 過濾聊天
    if string.find(clean_line, "^【") then return end

    -- ===== 啟動前檢查 (Otonashi) =====
    if s.phase == "checking_otonashi" then
        if string.find(clean_line, "他正在這個世界中", 1, true) then
            _G.IkkokuQuest.echo("✅ 音無爸爸確認存活！任務正式開始...")
            s.check_timer_active = false -- 標記檢查通過
            _G.IkkokuQuest.enter_sequence(s.run_id)
            return
        end
        return
    end

    -- ===== 全局體力偵測 =====
    if string.find(clean_line, "你精疲力竭了", 1, true) or
       string.find(clean_line, "你的移動力不足", 1, true) then
        _G.IkkokuQuest.echo("💤 體力不足，觸發自動恢復...")
        _G.IkkokuQuest.recover_stamina(s.run_id)
        return
    end

    if s.path_paused and string.find(clean_line, "你的體力逐漸地恢復", 1, true) then
        _G.IkkokuQuest.echo("✨ 體力已恢復！")
        s.path_paused = false
        if s.walking then
            _G.IkkokuQuest.safe_timer(1.0, "_G.IkkokuQuest.walk_resume")
        elseif s.phase == "exploring" then
            _G.IkkokuQuest.safe_timer(1.0, "_G.IkkokuQuest.retry_move")
        elseif s.phase == "waiting" then
            _G.IkkokuQuest.safe_timer(1.0, "_G.IkkokuQuest.wait_mob_check")
        end
        return
    end

    -- ===== 行走偵測 =====
    if s.walking then
        if not s.path_paused then
            if string.find(clean_line, "這個方向沒有出路", 1, true) then
                _G.IkkokuQuest.walk_advance()
                return
            end

            -- 門關著 (walk_path 中)
            if string.find(clean_line, "門是關著的", 1, true) then
                local cmd = s.path_queue[s.path_index]
                if cmd then
                    mud.send("op " .. cmd)
                    _G.IkkokuQuest.safe_timer(0.5, "_G.IkkokuQuest.walk_send")
                end
                return
            end

            if s.walk_expected and string.find(clean_line, "[出口:", 1, true) then
                _G.IkkokuQuest.walk_advance()
                return
            end
        end
    end

    -- ===== 召喚失敗自動重試 =====
    if string.find(clean_line, "你失敗了", 1, true) then
        local step = QUEST_STEPS[s.step_index]
        if step and (step.name == "find_otonashi_1" or step.name == "find_otonashi_2") then
             _G.IkkokuQuest.echo("🔄 召喚失敗，2秒後重試...")
             _G.IkkokuQuest.safe_timer(2.0, function() mud.send("c sum otonashi") end)
             return
        end
    end

    -- ===== 等待 mob (通用 + Kyokoo + Yotsuya 狀態檢測) =====
    if s.phase == "waiting" then
        -- 偵測四谷所在位置
        if string.find(clean_line, "牆縫中", 1, true) then
            s.yotsuya_pos = "gap"
        elseif string.find(clean_line, "五號房", 1, true) then
            s.yotsuya_pos = "room5"
        elseif string.find(clean_line, "四號房", 1, true) then
            s.yotsuya_pos = "room4"
        end

        local step = QUEST_STEPS[s.step_index]
        if step and step.target then
            if string.find(string.lower(clean_line), string.lower(step.target), 1, true) then
                _G.IkkokuQuest.echo("🎯 " .. step.target .. " 出現了！")
                s.phase = "acting"
                s.wait_timer_active = false
                _G.IkkokuQuest.execute_step_cmds(s.run_id)
                return
            end
        end
    end

    -- ===== Yotsuya 互動特殊處理 (跟我來/鑽了過去) =====
    if s.phase == "acting" then
        local step = QUEST_STEPS[s.step_index]
        if step and step.name == "find_yotsuya" then
            -- Debug: 顯示接收到的訊息
            if string.find(clean_line, "四谷") then
                _G.IkkokuQuest.echo("[Debug] Yotsuya msg: " .. clean_line)
            end

            -- 判定「跟我來」或「鑽了過去」
            if string.find(clean_line, "跟我來") or string.find(clean_line, "鑽了過去") then
                _G.IkkokuQuest.echo("🏃 偵測到四谷動作，立刻跟隨...")
                _G.IkkokuQuest.safe_timer(0.5, function()
                    _G.IkkokuQuest.echo("✨ 執行: squeeze -> talk yotsuya godai")
                    mud.send("squeeze")
                    mud.send("talk yotsuya godai")
                end)
                return
            end
        end

        -- ===== 通用 Expect 推進偵測 =====
        if step and step.expect and step.expect ~= "" then
            if not s.step_completed and string.find(clean_line, step.expect, 1, true) then
                _G.IkkokuQuest.echo("✨ 達成目標: " .. step.expect)
                s.step_completed = true
                _G.IkkokuQuest.safe_timer(0.5, "_G.IkkokuQuest.advance_step")
                return
            end
        end
    end

    -- ===== DFS 探索偵測 =====
    if s.phase == "exploring" and not s.walking then
        local step = QUEST_STEPS[s.step_index]
        if step then
            -- wait_kyokoo: 偵測「管理人室」房間名 + Kyokoo 是否在場
            if step.name == "wait_kyokoo" then
                if string.find(clean_line, "管理人室", 1, true) then
                    s.room_found = true
                end
                if string.find(string.lower(clean_line), "kyokoo", 1, true) then
                    s.target_in_room = true
                    s.target_line = clean_line
                end
            -- 一般 mob 偵測 (大小寫不敏感)
            elseif step.target then
                -- 一般 mob 偵測 (大小寫不敏感)
                if string.find(string.lower(clean_line), string.lower(step.target), 1, true) then
                    s.target_in_room = true
                    s.target_line = clean_line
                end
            end
        end

        -- [出口:] → 延遲處理
        if string.find(clean_line, "[出口:", 1, true) then
            s.explorer.last_exit_line = clean_line
            _G.IkkokuQuest.safe_timer(0.5, "_G.IkkokuQuest.explore_room_dispatch")
            return
        end

        -- 門關著
        if string.find(clean_line, "門是關著的", 1, true) then
            local exp = s.explorer
            if exp.pending then
                local cmd = exp.pending.type == "forward" and exp.pending.d.cmd or exp.pending.rev_cmd
                mud.send("op " .. cmd)
                _G.IkkokuQuest.safe_timer(1.0, "_G.IkkokuQuest.retry_move")
            end
            return
        end
    end
end

-- ============================================================
-- execute_step_cmds 触發
-- ============================================================

-- 覆寫 explore_room 中找到 target 的邏輯，對 go_squeeze 做特殊處理
local orig_execute = _G.IkkokuQuest.execute_step_cmds
_G.IkkokuQuest.execute_step_cmds = function(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    local step = QUEST_STEPS[s.step_index]
    if not step then return end

    -- 一般步驟
    s.phase = "acting"
    for _, cmd in ipairs(step.cmds) do
        mud.send(cmd)
    end
    
    -- 若有 expect，則不自動推進，等待 hook 偵測
    if step.expect and step.expect ~= "" then
        _G.IkkokuQuest.echo("⏳ 等待觸發條件: " .. step.expect)
    else
        _G.IkkokuQuest.safe_timer(3.0, "_G.IkkokuQuest.advance_step")
    end
end

-- ============================================================
-- 公開介面
-- ============================================================

function _G.IkkokuQuest.start()
    if _G.IkkokuQuest.state.running then
        _G.IkkokuQuest.echo("⚠️ 任務已在執行中")
        return
    end

    local s = _G.IkkokuQuest.state
    s.running = true
    s.run_id = s.run_id + 1
    s.phase = "entering"
    s.step_index = 1
    s.target_in_room = false
    s.target_line = nil
    s.room_found = false
    s.walking = false
    s.path_paused = false
    s.walk_expected = false
    s.wait_timer_active = false
    s.yotsuya_pos = "room5"
    s.yotsuya_dir = "east"
    s.check_timer_active = false

    s.explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        last_exit_line = nil,
        room_count = 0,
        laps = 0,
        doors_opened = false,
    }

    _G.IkkokuQuest.echo("═══════════════════════════════════════")
    _G.IkkokuQuest.echo("🔍 檢查音無爸爸是否已經重置...")
    s.phase = "checking_otonashi"
    s.check_timer_active = true
    mud.send("q otonashi")
    
    -- 3秒後若未通過檢查則中止
    _G.IkkokuQuest.safe_timer(3.0, function()
        if s.running and s.phase == "checking_otonashi" and s.check_timer_active then
             _G.IkkokuQuest.echo("❌ 音無爸爸還沒重置，任務取消。")
             _G.IkkokuQuest.stop()
        end
    end)
end

function _G.IkkokuQuest.enter_sequence(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end

    s.phase = "entering"  -- 進入正式任務階段
    _G.IkkokuQuest.echo("🏠 相聚一刻任務啟動！")
    _G.IkkokuQuest.echo("═══════════════════════════════════════")

    mud.send("repo")
    mud.send("wa")
    mud.send("recall")
    _G.IkkokuQuest.safe_timer(1.5, "_G.IkkokuQuest.enter_area")
end

function _G.IkkokuQuest.enter_area(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    _G.IkkokuQuest.walk_path(_G.IkkokuQuest.config.entry_path, "_G.IkkokuQuest.enter_done")
end

function _G.IkkokuQuest.enter_done(rid)
    if not check_run(rid) then return end
    local s = _G.IkkokuQuest.state
    if not s.running then return end
    _G.IkkokuQuest.echo("✅ 到達一刻館！")
    _G.IkkokuQuest.run_step(s.run_id)
end

function _G.IkkokuQuest.stop()
    local s = _G.IkkokuQuest.state
    s.running = false
    s.phase = "idle"
    s.walking = false
    _G.IkkokuQuest.echo("🛑 任務已停止")
end

function _G.IkkokuQuest.status()
    local s = _G.IkkokuQuest.state
    local step = QUEST_STEPS[s.step_index]
    _G.IkkokuQuest.echo("📊 狀態:")
    _G.IkkokuQuest.echo("   執行中: " .. (s.running and "是" or "否"))
    _G.IkkokuQuest.echo("   階段: " .. s.phase)
    _G.IkkokuQuest.echo("   步驟: " .. (step and step.name or "N/A") .. " (" .. s.step_index .. "/" .. #QUEST_STEPS .. ")")
    if step and step.target then
        _G.IkkokuQuest.echo("   目標: " .. step.target)
    end
    local exp = s.explorer
    _G.IkkokuQuest.echo("   已探索: " .. (exp and exp.room_count or 0) .. " 間")
    _G.IkkokuQuest.echo("   探索圈: " .. (exp and exp.laps or 0))
end

-- ============================================================
-- 載入訊息
-- ============================================================
local usage = [[
必備技能:
  summon
  refresh
指令:
  /lua IkkokuQuest.start()    啟動任務
  /lua IkkokuQuest.stop()     停止
  /lua IkkokuQuest.status()   查看狀態
流程:
  進入一刻館 → 等 Kyokoo → 依序找 mob 對話
  → 取得 bag/wine → 完成任務]]


mud.echo("========================================")
mud.echo("✅ IkkokuQuest 相聚一刻 v0.1 已載入")
mud.echo(usage)
mud.echo("========================================")
