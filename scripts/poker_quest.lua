-- ============================================================
-- PokerQuest - 撲克王國解謎任務自動腳本
-- ============================================================
-- 階段一：DFS 探索全圖，殺 spade 直到取得黃色石頭
-- 階段二：交石頭給方塊國王 → 取得甦醒咒語
-- 階段三：告訴黑桃王后 → 取得離開咒語
-- 階段四：前往紅心女王宮殿 → 完成任務
-- ============================================================
-- 使用: /lua PokerQuest.start()
-- 停止: /lua PokerQuest.stop()
-- ============================================================

_G.PokerQuest = _G.PokerQuest or {}

local string = string
local table = table
local ipairs = ipairs
local pairs = pairs
local tonumber = tonumber

-- ===== 方向映射 =====
local DIR_INFO = {
    {name="北", cmd="n", dx=0, dy=1, dz=0},
    {name="南", cmd="s", dx=0, dy=-1, dz=0},
    {name="東", cmd="e", dx=1, dy=0, dz=0},
    {name="西", cmd="w", dx=-1, dy=0, dz=0},
    {name="上", cmd="u", dx=0, dy=0, dz=1},
    {name="下", cmd="d", dx=0, dy=0, dz=-1},
}

local DIR_BY_NAME = {}  -- "北" → {name, cmd, dx, dy, dz}
local DIR_BY_CMD = {}   -- "n"  → {name, cmd, dx, dy, dz}
for _, d in ipairs(DIR_INFO) do
    DIR_BY_NAME[d.name] = d
    DIR_BY_CMD[d.cmd] = d
end

local REVERSE_CMD = {n="s", s="n", e="w", w="e", u="d", d="u"}

-- DFS 探索方向優先順序 (北→東→南→西→上→下)
local DIR_PRIORITY = {"北", "東", "南", "西", "上", "下"}

local function pos_key(pos)
    return pos.x .. "," .. pos.y .. "," .. pos.z
end

-- 解析 [出口: 北 東 南 西] → {"北", "東", "南", "西"}
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
_G.PokerQuest.config = {
    attack_cmd = "ear spade",       -- 攻擊 spade 的指令
    entry_path = "6s;2e;4u",        -- recall 到撲克王國的移動路線
    max_spades = 12,                 -- 撲克王國中 spade 總數
}

-- ===== 狀態 =====
_G.PokerQuest.state = {
    running = false,
    run_id = 0,
    phase = "idle",      -- idle / explore / pre_fight / fighting / clearing / looting / deliver / quest / done
    -- 行走佇列
    path_queue = {},
    path_index = 0,
    path_callback = nil,
    walking = false,
    path_paused = false,
    -- 探索器 (DFS)
    explorer = {
        pos = {x=0, y=0, z=0},   -- 當前座標 (起點=0,0,0)
        visited = {},              -- 已訪問座標 set {"x,y,z" = true}
        path = {},                 -- 路徑堆疊 {{cmd="n", rev="s"}, ...}
        exits = {},                -- 當前房間可用出口 {"北", "東", ...}
        pending = nil,             -- 待確認的移動 {type="forward/backtrack", ...}
        laps = 0,                  -- 探索圈數
        room_count = 0,            -- 已探索房間數
    },
    -- 戰鬥/物品
    got_stone = false,
    spade_in_room = false,
    kills = 0,
    spades_this_lap = 0,  -- 此圈找到的 spade 數
    -- 狀態數值解析
    status = {
        hp_cur = 0, hp_max = 0,
        ma_cur = 0, ma_max = 0,
        v_cur = 0, v_max = 0,
        p_cur = 0, p_max = 0,
    }
}

-- ===== run_id 檢查 =====
local function check_run(rid)
    if not rid then return true end
    return rid == _G.PokerQuest.state.run_id
end

-- ===== 訊息輸出 =====
function _G.PokerQuest.echo(msg)
    mud.echo("[PokerQuest] " .. msg)
end

-- ===== Timer Helper =====
function _G.PokerQuest.safe_timer(seconds, func_name, ...)
    local s = _G.PokerQuest.state
    if not s.running then return end
    local args = {...}
    table.insert(args, s.run_id)
    local serialized = {}
    for _, v in ipairs(args) do
        if type(v) == "string" then
            table.insert(serialized, string.format("%q", v))
        else
            table.insert(serialized, tostring(v))
        end
    end
    local code = func_name .. "(" .. table.concat(serialized, ", ") .. ")"
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
                    table.insert(result, actual)
                end
            else
                table.insert(result, cmd)
            end
        end
    end
    return result
end

-- ===== Prompt 驅動路徑行走 =====
function _G.PokerQuest.walk_path(str, callback)
    local s = _G.PokerQuest.state
    s.path_queue = parse_cmds(str)
    s.path_index = 1
    s.path_callback = callback
    s.path_paused = false
    s.walking = true
    _G.PokerQuest.walk_send(s.run_id)
end

function _G.PokerQuest.walk_send(rid)
    if not check_run(rid) then return end
    if not _G.PokerQuest.state.running then return end
    local s = _G.PokerQuest.state

    if s.path_index > #s.path_queue then
        s.walking = false
        s.path_queue = {}
        s.path_index = 0
        if s.path_callback then
            _G.PokerQuest.safe_timer(0.5, s.path_callback)
        end
        return
    end

    local cmd = s.path_queue[s.path_index]
    mud.send(cmd)
end

function _G.PokerQuest.walk_advance()
    local s = _G.PokerQuest.state
    s.path_index = s.path_index + 1
    _G.PokerQuest.safe_timer(0.05, "_G.PokerQuest.walk_send")
end

-- 檢查體力是否需要恢復 (基於 repo 數據)
local function needs_refresh()
    local s = _G.PokerQuest.state
    if s.status.v_max > 0 then
        return s.status.v_cur < (s.status.v_max * 0.7)
    end
    return true  -- 未知狀態，保守補體
end

-- 體力恢復指令
function _G.PokerQuest.recover_stamina(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    _G.PokerQuest.echo("✨ 施放 refresh...")
    mud.send("c ref")
end

-- 使用 repo 指令獲取精確狀態
function _G.PokerQuest.fetch_status(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase_after_repo = s.phase  -- 記住回來要回到哪個階段
    s.phase = "fetching_status"
    mud.send("repo")
end

function _G.PokerQuest.walk_resume()
    if not _G.PokerQuest.state.running then return end
    local s = _G.PokerQuest.state
    s.path_paused = false
    _G.PokerQuest.walk_send(s.run_id)
end

-- ============================================================
-- 階段一：DFS 探索全圖，殺 spade 取石頭
-- ============================================================

-- 初始化探索器
function _G.PokerQuest.start_explore(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "explore"
    s.explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        laps = s.explorer and s.explorer.laps or 0,
        room_count = 0,
    }
    _G.PokerQuest.echo("🔍 開始 DFS 探索撲克王國，尋找 spade...")
    s.spades_this_lap = 0  -- 重置此圈 spade 計數
    -- look 取得當前房間出口
    s.spade_in_room = false
    mud.send("l")
    -- hook 偵測 [出口:] → explore_room
end

-- 到達房間/確認移動 → 解析出口、標記已訪問、決定下一步
function _G.PokerQuest.explore_room(rid, exit_line)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    local exp = s.explorer

    -- 確認移動 (更新座標和路徑)
    if exp.pending then
        if exp.pending.type == "forward" then
            local d = exp.pending.d
            exp.pos = {x=exp.pos.x+d.dx, y=exp.pos.y+d.dy, z=exp.pos.z+d.dz}
            table.insert(exp.path, {cmd=d.cmd, rev=REVERSE_CMD[d.cmd]})
        elseif exp.pending.type == "backtrack" then
            local d = DIR_BY_CMD[exp.pending.rev_cmd]
            exp.pos = {x=exp.pos.x+d.dx, y=exp.pos.y+d.dy, z=exp.pos.z+d.dz}
            table.remove(exp.path) -- pop
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

    -- 檢查 spade
    if s.spade_in_room then
        _G.PokerQuest.echo("🎯 發現 spade！準備攻擊... (已探索 " .. exp.room_count .. " 間)")
        s.phase = "pre_fight"
        _G.PokerQuest.attack_spade(s.run_id)
    else
        _G.PokerQuest.explore_next(s.run_id)
    end
end

-- DFS 核心：找未訪問的鄰居 or 回溯
function _G.PokerQuest.explore_next(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    -- 已取得石頭 → 階段二
    if s.got_stone then
        _G.PokerQuest.echo("✅ 黃色石頭到手！進入階段二...")
        _G.PokerQuest.phase_deliver(s.run_id)
        return
    end

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
                -- 前進到未訪問房間
                exp.pending = {type="forward", d=d}
                s.spade_in_room = false
                s.explorer.last_exit_line = nil
                mud.send(d.cmd)
                return
            end
        end
    end

    -- 所有鄰居都已訪問 → 回溯
    if #exp.path > 0 then
        local last = exp.path[#exp.path]  -- peek (pop on confirm)
        exp.pending = {type="backtrack", rev_cmd=last.rev}
        s.spade_in_room = false
        s.explorer.last_exit_line = nil
        mud.send(last.rev)
    else
        -- 回到起點，全部探索完畢
        exp.laps = exp.laps + 1
        _G.PokerQuest.echo("🔄 第 " .. exp.laps .. " 圈完成！共探索 " .. exp.room_count .. " 個房間，此圈找到 " .. (s.spades_this_lap or 0) .. " 隻 spade")

        -- 此圈沒找到任何 spade → 可能全滅了，recall 重進讓 mob 重生
        -- 注意: #exp.path == 0 表示已回到起點(入口)，撲克王國只有入口能 recall
        if (s.spades_this_lap or 0) == 0 then
            _G.PokerQuest.echo("⚠️ 此圈未找到 spade，可能全滅！回城等待重生...")
            _G.PokerQuest.echo("📊 總擊殺: " .. s.kills .. " 隻")
            -- recall 回城
            s.phase = "entering"
            mud.send("c ref")
            mud.send("wa")
            mud.send("recall")
            -- 回城後開始在外部檢查重生
            _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.check_respawn")
        else
            -- 還有 spade 可找，重設已訪問開始新一圈
            exp.visited = {}
            exp.room_count = 0
            mud.send("c ref")
            _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.start_explore")
        end
    end
end

-- 探索中移動失敗重試
function _G.PokerQuest.explore_retry(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    local exp = s.explorer
    if exp.pending then
        local cmd = exp.pending.type == "forward" and exp.pending.d.cmd or exp.pending.rev_cmd
        s.spade_in_room = false
        s.explorer.last_exit_line = nil
        mud.send(cmd)
    end
end

-- ============================================================
-- 戰鬥系統
-- ============================================================

-- 攻擊 spade
function _G.PokerQuest.attack_spade(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "fighting"
    s.corpse_offset = 0  -- spade 死後額外死亡的 mob 數
    _G.PokerQuest.echo("⚔️ 攻擊 spade！")
    mud.send(_G.PokerQuest.config.attack_cmd)
end

-- 清場戰鬥結束 (超時 = 沒有更多敵人)
function _G.PokerQuest.combat_ended(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end
    -- 嚴格 phase guard: 只在 clearing 階段執行
    if s.phase ~= "clearing" then return end

    _G.PokerQuest.echo("✅ 戰鬥結束，清場完畢")

    -- 如果是 walk_path 途中被戰鬥打斷，恢復行走
    if s.walking and s.path_paused then
        _G.PokerQuest.echo("🚶 恢復行走...")
        s.path_paused = false
        s.phase = "deliver"  -- 恢復原本的 phase
        _G.PokerQuest.walk_send(s.run_id)
        return
    end

    -- 正常流程：撿取石頭
    s.phase = "looting"  -- 先轉 phase 防止重複觸發
    _G.PokerQuest.safe_timer(1.0, "_G.PokerQuest.loot_stone")
end

-- 清場中恢復移動力後繼續
function _G.PokerQuest.clear_continue(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "clearing"
    mud.send("ear")
    _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.combat_ended")
end

-- 擊殺後撿取石頭並祭獻屍體
function _G.PokerQuest.loot_stone(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "looting"
    -- 總屍體數 = spade + 其他被殺死的 mob
    local total = 1 + (s.corpse_offset or 0)
    
    _G.PokerQuest.echo("🔍 處理屍體 (共 " .. total .. " 具): 撿取石頭並祭獻...")
    -- 嘗試從所有屍體中取石頭 (不確定哪個是 spade 的)
    for i = total, 1, -1 do
        local target = i == 1 and "corpse" or (i .. ".corpse")
        mud.send("g stone " .. target)
    end
    
    -- 祭獻所有屍體
    for i = 1, total do
        mud.send("sac corpse")
    end

    _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.loot_check_result")
end

-- 撿取結果檢查
function _G.PokerQuest.loot_check_result(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end
    -- 嚴格 phase guard
    if s.phase ~= "looting" then return end

    if s.got_stone then
        _G.PokerQuest.echo("✅ 黃色石頭到手！進入階段二...")
        _G.PokerQuest.phase_deliver(s.run_id)
    else
        _G.PokerQuest.echo("❌ 這次沒掉石頭，繼續探索...")
        _G.PokerQuest.explore_next(s.run_id)
    end
end

-- ============================================================
-- 階段二：交石頭給方塊國王
-- ============================================================

function _G.PokerQuest.phase_deliver(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end
    -- 防止重複觸發
    if s.phase == "deliver" then return end

    s.phase = "deliver"
    _G.PokerQuest.echo("🚶 階段二：沿探索路徑回起點，再前往方塊國王...")

    -- 利用 explorer.path 堆疊反向回到起點
    local exp = s.explorer
    local return_cmds = {}
    for i = #exp.path, 1, -1 do
        return_cmds[#return_cmds + 1] = exp.path[i].rev
    end

    if #return_cmds > 0 then
        local path = table.concat(return_cmds, ";")
        _G.PokerQuest.echo("📍 沿原路 " .. #return_cmds .. " 步回到起點...")
        _G.PokerQuest.walk_path(path, "_G.PokerQuest.go_to_diamond_king")
    else
        _G.PokerQuest.go_to_diamond_king(s.run_id)
    end
end

function _G.PokerQuest.go_to_diamond_king(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "deliver"
    _G.PokerQuest.echo("📍 到達起點，前往方塊國王...")
    -- 從最南端到方塊國王： 6n → 2w → 2s → w
    _G.PokerQuest.walk_path("6n;2w;2s;w", "_G.PokerQuest.give_stone")
end

-- 交出石頭: 先 look 偵測方塊國王在第幾個 king
function _G.PokerQuest.give_stone(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    _G.PokerQuest.echo("🎁 偵測方塊國王位置...")
    s.phase = "detecting_king"
    s.king_count = 0
    s.diamond_king_index = nil
    mud.send("l")
    -- hook 會偵測國王順位並在 [出口:] 時執行 gi stone
end

-- 延遲後執行交付石頭 (由 safe_timer 呼叫)
function _G.PokerQuest.give_stone_now(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end
    if s.phase ~= "detecting_king" then return end

    local idx = s.diamond_king_index or 1
    local target = idx == 1 and "king" or (idx .. ".king")
    _G.PokerQuest.echo("🎁 交出石頭給 " .. target .. " (第" .. idx .. "個國王)...")
    s.phase = "deliver"
    mud.send("gi stone " .. target)
    _G.PokerQuest.safe_timer(4.0, "_G.PokerQuest.phase_queen")
end

-- ============================================================
-- 階段三：告訴黑桃王后
-- ============================================================
function _G.PokerQuest.phase_queen(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "quest"
    _G.PokerQuest.echo("🚶 階段三：前往黑桃王后...")
    -- 方塊國王 → 黑桃王后: e;2n;2e;n
    _G.PokerQuest.walk_path("e;2n;2e;n", "_G.PokerQuest.talk_queen")
end

function _G.PokerQuest.talk_queen(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    _G.PokerQuest.echo("💬 告訴黑桃王后甦醒咒語 goodmorning...")
    mud.send("say goodmorning")
    _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.phase_palace")
end

-- ============================================================
-- 階段四：前往宮殿完成任務
-- ============================================================
function _G.PokerQuest.phase_palace(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "quest"
    _G.PokerQuest.echo("🚶 階段四：前往紅心女王宮殿...")
    -- 黑桃王后 → 宮殿: 3s;3u
    _G.PokerQuest.walk_path("3s;3u", "_G.PokerQuest.say_leave")
end

function _G.PokerQuest.say_leave(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    _G.PokerQuest.echo("🎉 說出離開咒語 ireallywantleave！")
    mud.send("say ireallywantleave")
    _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.quest_complete")
end

function _G.PokerQuest.quest_complete(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    s.running = false
    s.phase = "done"
    local exp = s.explorer
    _G.PokerQuest.echo("═══════════════════════════════════════")
    _G.PokerQuest.echo("🎉 撲克王國任務完成！")
    _G.PokerQuest.echo("   擊殺 spade: " .. s.kills .. " 次")
    _G.PokerQuest.echo("   探索圈數: " .. exp.laps)
    _G.PokerQuest.echo("   探索房間: " .. exp.room_count .. " 間")
    _G.PokerQuest.echo("   獎勵: 幸福之杖 + 好運之杖")
    _G.PokerQuest.echo("═══════════════════════════════════════")
end

-- ============================================================
-- Server Message Hook
-- ============================================================
local base_hook = nil
if _G.on_server_message and not _G.PokerQuest.hook_installed then
    base_hook = _G.on_server_message
elseif _G.PokerQuest._base_hook then
    base_hook = _G.PokerQuest._base_hook
end
_G.PokerQuest._base_hook = base_hook

_G.on_server_message = function(line, clean_line)
    if base_hook then base_hook(line, clean_line) end
    if _G.PokerQuest and _G.PokerQuest.on_server_message then
        _G.PokerQuest.on_server_message(line, clean_line)
    end
end
_G.PokerQuest.hook_installed = true

function _G.PokerQuest.on_server_message(line, clean_line)
    if not _G.PokerQuest.state.running then return end

    local s = _G.PokerQuest.state
    if not clean_line or #clean_line < 3 then return end

    -- 過濾聊天頻道
    if string.find(clean_line, "^【") then return end

    -- ===== repo 狀態解析 =====
    -- 格式: 你報告自己的狀況: 2779/2741 生命力 2154/2154 精神力 1285/1364 移動力 1301/311 內力
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
        _G.PokerQuest.echo("📊 repo: HP=" .. (s.status.hp_cur or 0) .. "/" .. (s.status.hp_max or 0) .. 
            " MA=" .. (s.status.ma_cur or 0) .. "/" .. (s.status.ma_max or 0) .. 
            " V=" .. (s.status.v_cur or 0) .. "/" .. (s.status.v_max or 0))
        -- 如果是 fetching_status 階段，恢復原本的 phase
        if s.phase == "fetching_status" and s.phase_after_repo then
            s.phase = s.phase_after_repo
            s.phase_after_repo = nil
        end
        return
    end

    -- ===== 行走中的偵測 (walk_path 用) =====
    if s.walking and not s.path_paused then
        if string.find(clean_line, "你精疲力竭了", 1, true) or
           string.find(clean_line, "你的移動力不足", 1, true) then
            s.path_paused = true
            _G.PokerQuest.echo("💤 體力不足，自動恢復...")
            _G.PokerQuest.recover_stamina(s.run_id)
            return
        end
        -- 撞牆 = 跳過此步
        if string.find(clean_line, "這個方向沒有路", 1, true) then
            _G.PokerQuest.walk_advance()
            return
        end
        -- 戰鬥中無法移動 → 清場後繼續
        if string.find(clean_line, "身陷戰鬥中", 1, true) then
            s.path_paused = true
            _G.PokerQuest.echo("⚔️ 正在戰鬥中，清場後繼續移動...")
            s.phase = "clearing"
            mud.send("ear")
            return
        end
        if string.find(clean_line, "[出口:", 1, true) then
            _G.PokerQuest.walk_advance()
            return
        end
    end

    -- 體力恢復偵測
    if s.path_paused and string.find(clean_line, "你的體力逐漸地恢復", 1, true) then
        _G.PokerQuest.echo("✅ 體力已恢復，繼續前進...")
        _G.PokerQuest.safe_timer(0.5, "_G.PokerQuest.walk_resume")
        return
    end

    -- ===== 探索模式：偵測房間內容 =====
    if s.phase == "explore" then
        -- 偵測 spade
        if string.find(clean_line, "小黑桃", 1, true) or
           string.find(clean_line, "spade", 1, true) then
            s.spade_in_room = true
            -- 不 return，讓 [出口:] 也能被偵測
        end

        -- [出口:] → 觸發探索
        if string.find(clean_line, "[出口:", 1, true) then
            -- 儲存出口行
            s.explorer.last_exit_line = clean_line
            -- 重要：延後 0.5s 執行，確保怪物資訊已進入（怪物資訊通常在出口行後 1-2 行）
            _G.PokerQuest.safe_timer(0.5, "_G.PokerQuest.explore_room_dispatch")
            return
        end

        -- 體力不足
        if string.find(clean_line, "你精疲力竭了", 1, true) or
           string.find(clean_line, "你的移動力不足", 1, true) then
            _G.PokerQuest.echo("💤 體力不足，施放 refresh...")
            mud.send("c ref")
            mud.send("c ref")
            _G.PokerQuest.safe_timer(4.0, "_G.PokerQuest.explore_retry")
            return
        end

        -- 沒有路 (理論上不應發生，但以防萬一)
        if string.find(clean_line, "這個方向沒有路", 1, true) then
            _G.PokerQuest.echo("🚫 方向無效，重新探索...")
            s.explorer.pending = nil
            _G.PokerQuest.explore_next(s.run_id)
            return
        end
    end

    -- ===== 等待 spade 重生 =====
    if s.phase == "waiting_respawn" then
        -- q 12.spade 回應: 有資訊 = 存在
        if string.find(clean_line, "Spade", 1, true) and
           (string.find(clean_line, "身體", 1, true) or
            string.find(clean_line, "工作", 1, true) or
            string.find(clean_line, "生命力", 1, true) or
            string.find(clean_line, "黑桃", 1, true)) then
            s.respawn_confirmed = true
            return
        end
    end

    -- ===== 戰鬥前恢復 =====
    if s.phase == "pre_fight" then
        if string.find(clean_line, "你的體力逐漸地恢復", 1, true) then
            return -- 等 timer 觸發 attack_spade
        end
    end

    -- ===== 戰鬥偵測 =====
    if s.phase == "fighting" then
        -- 移動力不足 (偵測失敗訊息)
        if string.find(clean_line, "移動力不足", 1, true) then
            _G.PokerQuest.echo("⚡ 移動力不足以戰鬥，即時補體...")
            _G.PokerQuest.recover_stamina(s.run_id)
            _G.PokerQuest.safe_timer(2.0, "_G.PokerQuest.attack_spade")
            return
        end

        -- 擊殺 spade → 清場
        if string.find(clean_line, "魂歸西天了", 1, true) and
           string.find(clean_line, "Spade", 1, true) then
            s.kills = s.kills + 1
            s.spades_this_lap = (s.spades_this_lap or 0) + 1
            s.corpse_offset = 0  -- 重置額外死亡計數
            _G.PokerQuest.echo("💀 擊殺 spade #" .. s.kills .. "，檢查是否還有其他敵人...")
            s.phase = "clearing"
            _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.combat_ended")
            return
        end

        -- 其他 mob 死了 (非 spade) → 追蹤 corpse 偏移
        if string.find(clean_line, "魂歸西天了", 1, true) then
            s.corpse_offset = (s.corpse_offset or 0) + 1
            return
        end
    end

    -- ===== 清場階段 =====
    if s.phase == "clearing" then
        -- 戰鬥回合偵測 → 繼續打
        if string.find(clean_line, "你正蓄勢待發", 1, true) or
           string.find(clean_line, "你心裡正盤算著", 1, true) then
            mud.send("ear")
            _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.combat_ended")
            return
        end

        -- 更多敵人死亡 (增加 corpse 偏移) 並重置清場計時器
        if string.find(clean_line, "魂歸西天了", 1, true) then
            s.corpse_offset = (s.corpse_offset or 0) + 1
            _G.PokerQuest.safe_timer(3.0, "_G.PokerQuest.combat_ended")
            return
        end

        -- 移動力不足
        if string.find(clean_line, "移動力不足", 1, true) then
            mud.send("c ref")
            _G.PokerQuest.safe_timer(4.0, "_G.PokerQuest.clear_continue")
            return
        end
    end

    -- ===== looting 階段也追蹤額外死亡=====
    if s.phase == "looting" then
        if string.find(clean_line, "魂歸西天了", 1, true) then
            s.corpse_offset = (s.corpse_offset or 0) + 1
            return
        end
    end

    -- ===== 撿取偵測 =====
    -- ===== 偵測方塊國王順位 =====
    if s.phase == "detecting_king" then
        -- 計算國王出現順序
        if string.find(clean_line, "國王", 1, true) then
            s.king_count = (s.king_count or 0) + 1
            if string.find(clean_line, "方塊國王", 1, true) then
                s.diamond_king_index = s.king_count
            end
        end
        -- [出口:] → 房間載入完成，重置計數並延遲執行交付
        if string.find(clean_line, "[出口:", 1, true) then
            -- 重置計數! 防止 walk_path 殘留輸出污染
            s.king_count = 0
            s.diamond_king_index = nil
            -- 延後 0.5s 等待國王名稱行載入
            _G.PokerQuest.safe_timer(0.5, "_G.PokerQuest.give_stone_now")
            return
        end
    end

    -- ===== 撿取偵測 =====
    if s.phase == "looting" then
        if string.find(clean_line, "黃色石頭", 1, true) and
           string.find(clean_line, "你從", 1, true) then
            s.got_stone = true
            _G.PokerQuest.echo("🎉 取得黃色石頭！！")
            return
        end
    end
end

-- explore_room 的 dispatch (從 hook timer 呼叫)
function _G.PokerQuest.explore_room_dispatch(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end
    _G.PokerQuest.explore_room(rid, s.explorer.last_exit_line or "")
end

-- ============================================================
-- 公開介面
-- ============================================================

function _G.PokerQuest.start()
    if _G.PokerQuest.state.running then
        _G.PokerQuest.echo("⚠️ 任務已在執行中")
        return
    end

    local s = _G.PokerQuest.state
    s.running = true
    s.run_id = (s.run_id or 0) + 1
    s.phase = "idle"
    s.kills = 0
    s.got_stone = false
    s.spade_in_room = false
    s.walking = false
    s.path_paused = false
    s.explorer = {
        pos = {x=0, y=0, z=0},
        visited = {},
        path = {},
        exits = {},
        pending = nil,
        laps = 0,
        room_count = 0,
    }

    _G.PokerQuest.echo("═══════════════════════════════════════")
    _G.PokerQuest.echo("🃏 撲克王國解謎任務 開始！")
    _G.PokerQuest.echo("═══════════════════════════════════════")
    _G.PokerQuest.echo("   階段一: recall → 進入撲克 → DFS 探索全圖殺 spade")
    _G.PokerQuest.echo("   階段二: 交石頭給方塊國王")
    _G.PokerQuest.echo("   階段三: 告訴黑桃王后解咒")
    _G.PokerQuest.echo("   階段四: 宮殿離開，完成任務")

    -- wa → repo → recall → walk_path 進入撲克王國
    s.phase = "entering"
    _G.PokerQuest.echo("🚶 repo → recall → 前往撲克王國入口...")
    mud.send("repo")  -- 獲取初始狀態
    mud.send("wa")
    mud.send("recall")
    _G.PokerQuest.safe_timer(1.5, "_G.PokerQuest.enter_kingdom")
end

-- recall 完成後走到撲克王國
function _G.PokerQuest.enter_kingdom(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    _G.PokerQuest.walk_path(_G.PokerQuest.config.entry_path, "_G.PokerQuest.enter_kingdom_wake")
end

-- 到達凍原山頂後醒來
function _G.PokerQuest.enter_kingdom_wake(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    mud.send("wa")
    -- 到達內部入口，直接開始探索
    _G.PokerQuest.safe_timer(1.5, "_G.PokerQuest.start_explore")
end

-- 檢查 spade 是否全部重生
function _G.PokerQuest.check_respawn(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end

    s.phase = "waiting_respawn"
    s.respawn_confirmed = false
    _G.PokerQuest.echo("⏳ 檢查 spade 是否已重生 (q 12.spade)...")
    mud.send("q 12.spade")
    -- hook 會偵測回應
    _G.PokerQuest.safe_timer(5.0, "_G.PokerQuest.check_respawn_result")
end

-- 檢查 q 12.spade 的結果
function _G.PokerQuest.check_respawn_result(rid)
    if not check_run(rid) then return end
    local s = _G.PokerQuest.state
    if not s.running then return end
    if s.phase ~= "waiting_respawn" then return end

    if s.respawn_confirmed then
        _G.PokerQuest.echo("✅ spade 已全部重生！執行進入路徑...")
        s.kills = 0  -- 重置擊殺計數
        _G.PokerQuest.enter_kingdom(s.run_id)
    else
        _G.PokerQuest.echo("⏳ spade 尚未全部重生，30 秒後再檢查...")
        _G.PokerQuest.safe_timer(30.0, "_G.PokerQuest.check_respawn")
    end
end

function _G.PokerQuest.stop()
    _G.PokerQuest.state.running = false
    _G.PokerQuest.state.phase = "idle"
    _G.PokerQuest.state.walking = false
    _G.PokerQuest.echo("🛑 任務已停止")
end

function _G.PokerQuest.status()
    local s = _G.PokerQuest.state
    local exp = s.explorer
    _G.PokerQuest.echo("📊 狀態:")
    _G.PokerQuest.echo("   執行中: " .. (s.running and "是" or "否"))
    _G.PokerQuest.echo("   階段: " .. s.phase)
    _G.PokerQuest.echo("   擊殺: " .. s.kills)
    if s.status.v_max > 0 then
        _G.PokerQuest.echo("   體力: " .. s.status.v_cur .. "/" .. s.status.v_max .. " (" .. math.floor(s.status.v_cur / s.status.v_max * 100) .. "%)")
    end
    _G.PokerQuest.echo("   石頭: " .. (s.got_stone and "已取得" or "未取得"))
    _G.PokerQuest.echo("   探索圈: " .. (exp and exp.laps or 0))
    _G.PokerQuest.echo("   已探索: " .. (exp and exp.room_count or 0) .. " 間")
    _G.PokerQuest.echo("   路徑深度: " .. (exp and #exp.path or 0))
    if exp and exp.pos then
        _G.PokerQuest.echo("   座標: " .. pos_key(exp.pos))
    end
end

-- ===== Help 註冊 =====
local usage = [[
指令:
  1. 啟動: /lua PokerQuest.start()
  2. 停止: /lua PokerQuest.stop()
  3. 狀態: /lua PokerQuest.status()
流程:
  recall → 進入撲克王國
  → DFS 探索全圖殺 spade (直到取得黃色石頭)
  → 交給方塊國王換甦醒咒語
  → 告訴黑桃王后解咒 → 到宮殿離開
獎勵: 幸福之杖 + 好運之杖]]

mud.echo("========================================")
mud.echo("✅ PokerQuest 撲克王國解謎任務v0.1 已載入")
mud.echo(usage)
mud.echo("========================================")

_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["PokerQuest"] = {
    desc = "撲克王國解謎任務自動腳本 (DFS 探索)",
    usage = usage
}
