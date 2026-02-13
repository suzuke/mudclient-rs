-- ============================================================
-- MobFinder - 指定 Mob 搜尋腳本
-- ============================================================
-- 使用 DFS 探索指定區域，找到目標 mob 後通知
-- ============================================================
-- 使用: /lua MobFinder.start()          -- 用預設 config
--       /lua MobFinder.start("queen")   -- 覆寫 target
--       /lua MobFinder.stop()
--       /lua MobFinder.status()
-- ============================================================

_G.MobFinder = _G.MobFinder or {}

local string = string
local table = table
local ipairs = ipairs
local pairs = pairs
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
_G.MobFinder.config = {
    target = "otonashi",          -- mob 關鍵字 (比對房間內容)
    entry_path = "6w;3n;enter ikkoku",  -- recall 到目標區域的路線
    enter_cmds = {},               -- 進入後額外指令 (如 {"wa"} 用於撲克王國)
    on_found = nil,                -- 找到後的回呼函數名 (未來擴充)
    max_laps = 5,                  -- 最大探索圈數 (mob 會移動時需多圈)
}

-- ===== 狀態 =====
_G.MobFinder.state = {
    running = false,
    run_id = 0,
    phase = "idle",      -- idle / entering / explore / found / done
    -- 行走佇列
    path_queue = {},
    path_index = 0,
    path_callback = nil,
    walking = false,
    path_paused = false,
    walk_expected = false,  -- 是否期待 [出口:] 回應
    -- 探索器 (DFS)
    explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        last_exit_line = nil,
        room_count = 0,
    },
    -- mob 偵測
    target_in_room = false,
    target_line = nil,    -- 找到的那一行原文
    -- 狀態數值 (repo)
    status = {
        hp_cur = 0, hp_max = 0,
        ma_cur = 0, ma_max = 0,
        v_cur = 0, v_max = 0,
        p_cur = 0, p_max = 0,
    },
}

-- ===== run_id 檢查 =====
local function check_run(rid)
    if not rid then return true end
    return rid == _G.MobFinder.state.run_id
end

-- ===== 訊息輸出 =====
function _G.MobFinder.echo(msg)
    mud.echo("[MobFinder] " .. msg)
end

-- ===== Timer Helper =====
function _G.MobFinder.safe_timer(seconds, func_name)
    local s = _G.MobFinder.state
    if not s.running then return end
    local code = func_name .. "(" .. s.run_id .. ")"
    mud.timer(seconds, code)
end

-- ===== 指令解析 (支援 3n 展開) =====
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

-- ===== Prompt 驅動路徑行走 =====
function _G.MobFinder.walk_path(str, callback)
    local s = _G.MobFinder.state
    s.path_queue = parse_cmds(str)
    s.path_index = 1
    s.path_callback = callback
    s.path_paused = false
    s.walking = true
    _G.MobFinder.walk_send(s.run_id)
end

function _G.MobFinder.walk_send(rid)
    if not check_run(rid) then return end
    if not _G.MobFinder.state.running then return end
    local s = _G.MobFinder.state

    if s.path_index > #s.path_queue then
        s.walking = false
        s.walk_expected = false
        s.path_queue = {}
        s.path_index = 0
        if s.path_callback then
            _G.MobFinder.safe_timer(0.5, s.path_callback)
        end
        return
    end

    local cmd = s.path_queue[s.path_index]
    s.walk_expected = true
    mud.send(cmd)
end

function _G.MobFinder.walk_advance()
    local s = _G.MobFinder.state
    s.walk_expected = false
    s.path_index = s.path_index + 1
    _G.MobFinder.safe_timer(0.05, "_G.MobFinder.walk_send")
end

-- 體力恢復
function _G.MobFinder.recover_stamina(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end

    _G.MobFinder.echo("✨ 施放 refresh...")
    mud.send("c ref")
end

function _G.MobFinder.walk_resume()
    if not _G.MobFinder.state.running then return end
    local s = _G.MobFinder.state
    s.path_paused = false
    _G.MobFinder.walk_send(s.run_id)
end

-- ============================================================
-- DFS 探索
-- ============================================================

function _G.MobFinder.start_explore(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end

    s.phase = "explore"
    s.target_in_room = false
    s.explorer.last_exit_line = nil
    -- 先嘗試開所有方向的門，再 look 取得更新後的出口
    mud.send("op n")
    mud.send("op s")
    mud.send("op e")
    mud.send("op w")
    mud.send("op u")
    mud.send("op d")
    mud.send("l")
end

-- 處理房間資訊 (由 [出口:] 延遲觸發)
function _G.MobFinder.explore_room_dispatch(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end
    if s.phase ~= "explore" then return end

    local exp = s.explorer
    -- 前進到新房間時: 先開門再重新 look
    if exp.pending and exp.pending.type == "forward" and not exp.doors_opened then
        exp.doors_opened = true
        mud.send("op n")
        mud.send("op s")
        mud.send("op e")
        mud.send("op w")
        mud.send("op u")
        mud.send("op d")
        -- 重新 look 以取得包含新開門的出口
        s.target_in_room = false
        mud.send("l")
        -- 下次 [出口:] 觸發時 doors_opened=true → 直接處理
        return
    end

    exp.doors_opened = false
    _G.MobFinder.explore_room(rid, s.explorer.last_exit_line or "")
end

function _G.MobFinder.explore_room(rid, exit_line)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end

    local exp = s.explorer

    -- 確認座標 (pending 移動)
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
            if #exp.path > 0 then
                table.remove(exp.path)
            end
        end
        exp.pending = nil
    end

    -- 解析出口
    exp.exits = parse_exits(exit_line)

    -- 標記已訪問
    local key = pos_key(exp.pos)
    if not exp.visited[key] then
        exp.visited[key] = true
        exp.room_count = exp.room_count + 1
    end

    -- 檢查目標 mob
    if s.target_in_room then
        _G.MobFinder.echo("🎯 找到目標！(" .. _G.MobFinder.config.target .. ")")
        if s.target_line then
            _G.MobFinder.echo("   " .. s.target_line)
        end
        _G.MobFinder.echo("   座標: " .. pos_key(exp.pos))
        _G.MobFinder.echo("   已探索 " .. exp.room_count .. " 間房間")

        -- 未來擴充: 執行 on_found 回呼
        if _G.MobFinder.config.on_found then
            _G.MobFinder.safe_timer(0.5, _G.MobFinder.config.on_found)
        else
            _G.MobFinder.stop()
        end
        return
    end

    _G.MobFinder.explore_next(s.run_id)
end

-- DFS 核心
function _G.MobFinder.explore_next(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end

    s.phase = "explore"
    local exp = s.explorer

    -- 找可用出口中未訪問的鄰居
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

    -- 所有鄰居都已訪問 → 回溯
    if #exp.path > 0 then
        local last = exp.path[#exp.path]
        exp.pending = {type="backtrack", rev_cmd=last.rev}
        s.target_in_room = false
        s.explorer.last_exit_line = nil
        mud.send(last.rev)
    else
        -- 回到起點，全部探索完畢
        exp.laps = (exp.laps or 0) + 1
        local max_laps = _G.MobFinder.config.max_laps or 5
        _G.MobFinder.echo("🔄 第 " .. exp.laps .. " 圈探索完畢！共 " .. exp.room_count .. " 間房間")

        if exp.laps >= max_laps then
            _G.MobFinder.echo("❌ 已探索 " .. max_laps .. " 圈，未找到目標: " .. _G.MobFinder.config.target)
            _G.MobFinder.stop()
        else
            _G.MobFinder.echo("🔍 Mob 可能已移動，開始第 " .. (exp.laps + 1) .. " 圈...")
            -- 重置 visited 但保留 room_count 與位置
            exp.visited = {}
            exp.visited[pos_key(exp.pos)] = true
            s.target_in_room = false
            _G.MobFinder.explore_next(s.run_id)
        end
    end
end

-- 撞牆重試
function _G.MobFinder.retry_move(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    local exp = s.explorer
    if exp.pending then
        local cmd = exp.pending.type == "forward" and exp.pending.d.cmd or exp.pending.rev_cmd
        s.target_in_room = false
        s.explorer.last_exit_line = nil
        mud.send(cmd)
    end
end

-- ============================================================
-- Server Hook (使用 _G.on_server_message)
-- ============================================================
local base_hook = nil
if _G.on_server_message and not _G.MobFinder.hook_installed then
    base_hook = _G.on_server_message
elseif _G.MobFinder._base_hook then
    base_hook = _G.MobFinder._base_hook
end
_G.MobFinder._base_hook = base_hook

_G.on_server_message = function(line, clean_line)
    if base_hook then base_hook(line, clean_line) end
    if _G.MobFinder and _G.MobFinder.on_server_message then
        _G.MobFinder.on_server_message(line, clean_line)
    end
end
_G.MobFinder.hook_installed = true

function _G.MobFinder.on_server_message(line, clean_line)
    if not _G.MobFinder.state.running then return end

    local s = _G.MobFinder.state
    if not clean_line or #clean_line < 3 then return end

    -- 過濾聊天頻道
    if string.find(clean_line, "^【") then return end

    -- ===== repo 狀態解析 =====
    if string.find(clean_line, "你報告自己的狀況", 1, true) then
        local hp_cur, hp_max = string.match(clean_line, "(%d+)/(%d+)%s*生命力")
        local ma_cur, ma_max = string.match(clean_line, "(%d+)/(%d+)%s*精神力")
        local v_cur, v_max = string.match(clean_line, "(%d+)/(%d+)%s*移動力")
        local p_cur, p_max = string.match(clean_line, "(%d+)/(%d+)%s*內力")
        if hp_cur then
            s.status.hp_cur = tonumber(hp_cur)
            s.status.hp_max = tonumber(hp_max)
        end
        if ma_cur then
            s.status.ma_cur = tonumber(ma_cur)
            s.status.ma_max = tonumber(ma_max)
        end
        if v_cur then
            s.status.v_cur = tonumber(v_cur)
            s.status.v_max = tonumber(v_max)
        end
        if p_cur then
            s.status.p_cur = tonumber(p_cur)
            s.status.p_max = tonumber(p_max)
        end
        return
    end

    -- ===== 行走偵測 (walk_path) =====
    if s.walking then
        -- refresh 完畢恢復行走 (必須在 path_paused 檢查之前)
        if s.path_paused and string.find(clean_line, "你的體力逐漸地恢復", 1, true) then
            _G.MobFinder.safe_timer(1.0, "_G.MobFinder.walk_resume")
            return
        end

        if not s.path_paused then
            -- 體力不足
            if string.find(clean_line, "你精疲力竭了", 1, true) or
               string.find(clean_line, "你的移動力不足", 1, true) then
                s.path_paused = true
                _G.MobFinder.echo("💤 體力不足，自動恢復...")
                _G.MobFinder.recover_stamina(s.run_id)
                return
            end

            -- 撞牆
            if string.find(clean_line, "這個方向沒有出路", 1, true) then
                _G.MobFinder.walk_advance()
                return
            end

            -- [出口:] → 到達新房間 (只在期待回應時才推進)
            if s.walk_expected and string.find(clean_line, "[出口:", 1, true) then
                _G.MobFinder.walk_advance()
                return
            end
        end
    end

    -- ===== 探索偵測 =====
    if s.phase == "explore" and not s.walking then
        -- 偵測目標 mob (大小寫不敏感)
        if string.find(string.lower(clean_line), string.lower(_G.MobFinder.config.target), 1, true) then
            s.target_in_room = true
            s.target_line = clean_line
        end

        -- spade 來了 (忽略，非戰鬥腳本)
        -- 不處理戰鬥，只做移動

        -- [出口:] → 延遲 0.5s 確保 mob 資訊載入
        if string.find(clean_line, "[出口:", 1, true) then
            s.explorer.last_exit_line = clean_line
            _G.MobFinder.safe_timer(0.5, "_G.MobFinder.explore_room_dispatch")
            return
        end

        -- 門是關著的 → 自動開門並重試
        if string.find(clean_line, "門是關著的", 1, true) then
            local exp = s.explorer
            if exp.pending then
                local cmd = exp.pending.type == "forward" and exp.pending.d.cmd or exp.pending.rev_cmd
                _G.MobFinder.echo("🚪 門關著，自動開門 (op " .. cmd .. ")...")
                mud.send("op " .. cmd)
                _G.MobFinder.safe_timer(1.0, "_G.MobFinder.retry_move")
            end
            return
        end

        -- 體力不足 (探索中)
        if string.find(clean_line, "你精疲力竭了", 1, true) or
           string.find(clean_line, "你的移動力不足", 1, true) then
            _G.MobFinder.echo("💤 體力不足，施放 refresh...")
            mud.send("c ref")
            _G.MobFinder.safe_timer(3.0, "_G.MobFinder.retry_move")
            return
        end
    end
end

-- ============================================================
-- 公開介面
-- ============================================================

function _G.MobFinder.start(target)
    if _G.MobFinder.state.running then
        _G.MobFinder.echo("⚠️ 搜尋已在執行中")
        return
    end

    -- 覆寫 target
    if target then
        _G.MobFinder.config.target = target
    end

    local s = _G.MobFinder.state
    s.running = true
    s.run_id = s.run_id + 1
    s.phase = "entering"
    s.target_in_room = false
    s.target_line = nil
    s.walking = false
    s.path_paused = false
    s.walk_expected = false

    -- 重置探索器
    s.explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        last_exit_line = nil,
        room_count = 0,
    }

    _G.MobFinder.echo("═══════════════════════════════════════")
    _G.MobFinder.echo("🔍 MobFinder 啟動！")
    _G.MobFinder.echo("   目標: " .. _G.MobFinder.config.target)
    _G.MobFinder.echo("   路徑: " .. _G.MobFinder.config.entry_path)
    _G.MobFinder.echo("═══════════════════════════════════════")

    -- repo → recall → 進入
    mud.send("repo")
    mud.send("wa")
    mud.send("recall")
    _G.MobFinder.safe_timer(1.5, "_G.MobFinder.enter_area")
end

function _G.MobFinder.enter_area(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end

    _G.MobFinder.walk_path(_G.MobFinder.config.entry_path, "_G.MobFinder.enter_area_done")
end

function _G.MobFinder.enter_area_done(rid)
    if not check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end

    -- 執行進入後額外指令 (如 wa)
    local cmds = _G.MobFinder.config.enter_cmds or {}
    for _, cmd in ipairs(cmds) do
        mud.send(cmd)
    end
    local delay = #cmds > 0 and 1.5 or 0.5
    _G.MobFinder.safe_timer(delay, "_G.MobFinder.start_explore")
end

function _G.MobFinder.stop()
    _G.MobFinder.state.running = false
    _G.MobFinder.state.phase = "idle"
    _G.MobFinder.state.walking = false
    _G.MobFinder.echo("🛑 搜尋已停止")
end

function _G.MobFinder.status()
    local s = _G.MobFinder.state
    local exp = s.explorer
    _G.MobFinder.echo("📊 狀態:")
    _G.MobFinder.echo("   執行中: " .. (s.running and "是" or "否"))
    _G.MobFinder.echo("   階段: " .. s.phase)
    _G.MobFinder.echo("   目標: " .. _G.MobFinder.config.target)
    if s.status.v_max > 0 then
        _G.MobFinder.echo("   體力: " .. s.status.v_cur .. "/" .. s.status.v_max .. " (" .. math.floor(s.status.v_cur / s.status.v_max * 100) .. "%)")
    end
    _G.MobFinder.echo("   已探索: " .. (exp and exp.room_count or 0) .. " 間")
    _G.MobFinder.echo("   路徑深度: " .. (exp and #exp.path or 0))
    if exp and exp.pos then
        _G.MobFinder.echo("   座標: " .. pos_key(exp.pos))
    end
end

-- ============================================================
-- 載入訊息
-- ============================================================

local usage = [[
指令:
  1. 啟動: /lua MobFinder.start()
  2. 指定: /lua MobFinder.start("queen")
  3. 停止: /lua MobFinder.stop()
  4. 狀態: /lua MobFinder.status()
流程:
  recall → 進入目標區域 → DFS 探索 → 找到目標 mob → 通知並停止]]

mud.echo("========================================")
mud.echo("✅ MobFinder 指定 Mob 搜尋 v0.1 已載入")
mud.echo(usage)
mud.echo("========================================")

_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["MobFinder"] = {
    desc = "指定 Mob 搜尋腳本 (DFS 探索)",
    usage = usage
}
