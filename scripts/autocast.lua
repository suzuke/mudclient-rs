-- autocast.lua
-- 自動練功腳本 (事件驅動版 v2.0)
-- 使用狀態機管理 stopped → initializing → casting → sleeping 循環
-- 載入: 自動
-- 使用: /lua AutoCast.start("cast 'summon' boy")
-- 停止: /lua AutoCast.stop()

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("AutoCast cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

_G.AutoCast = _G.AutoCast or {}

-- 設定
_G.AutoCast.config = {
    command = "cast 'summon' boy",
    debug = false
}

-- 內部狀態（非狀態機管理的部分）
_G.AutoCast.state = {
    max_mp = 0,
    current_mp = 0,
    run_id = 0,
    check_count = 0,
}

-- ===== 狀態機定義 =====
mud.state_machine("autocast", {
    initial = "stopped",
    states = {
        stopped = {},
        initializing = {
            enter = "mud.echo('🔍 AutoCast 檢查狀態...'); mud.send('score')",
        },
        casting = {
            enter = "mud.echo('⚡ AutoCast 開始施法')",
        },
        sleeping = {
            enter = "mud.echo('💤 AutoCast 進入睡眠回魔'); mud.send('sleep')",
        },
    },
    transitions = {
        { from = "stopped",      event = "ac_start",     to = "initializing" },
        { from = "initializing", event = "ac_mp_ok",     to = "casting" },
        { from = "initializing", event = "ac_mp_low",    to = "sleeping" },
        { from = "initializing", event = "ac_exhausted", to = "sleeping" },
        { from = "casting",      event = "ac_exhausted", to = "sleeping" },
        { from = "sleeping",     event = "ac_mp_full",   to = "casting" },
        { from = "initializing", event = "ac_stop",      to = "stopped" },
        { from = "casting",      event = "ac_stop",      to = "stopped" },
        { from = "sleeping",     event = "ac_stop",      to = "stopped" },
    },
})

-- ===== 伺服器訊息偵測 → 發送事件 =====
MudUtils.register_hook("AutoCast", function(line, clean_line)
    local sm_state = mud.sm_current("autocast")
    if not sm_state or sm_state == "stopped" then return end
    if not clean_line then return end
    clean_line = string.match(clean_line, "^%s*(.-)%s*$")

    -- 偵測精神力耗盡
    if string.find(clean_line, "耗盡") and string.find(clean_line, "精神力") then
        if sm_state ~= "sleeping" then
            mud.emit("ac_exhausted")
        end
        return
    end

    -- 偵測 Score 中的精神力數值
    if sm_state == "sleeping" or sm_state == "initializing" then
        local mp, max_mp = string.match(clean_line, "精神力[^%d]*(%d+)[^%d]*(%d+)")
        if mp and max_mp then
            local current = tonumber(mp)
            local total = tonumber(max_mp)
            _G.AutoCast.state.current_mp = current
            _G.AutoCast.state.max_mp = total

            if _G.AutoCast.config.debug then
                mud.echo(string.format("[Debug] MP=%d/%d (%.0f%%), State=%s",
                    current, total, current/total*100, sm_state))
            end

            local percent = (total > 0) and (current / total * 100) or 0

            if sm_state == "initializing" then
                mud.emit(percent < 10 and "ac_mp_low" or "ac_mp_ok")
            elseif sm_state == "sleeping" and percent >= 98 then
                mud.emit("ac_mp_full")
            end
            return
        end
    end

    -- "太睏了" → 確認已在睡眠
    if string.find(clean_line, "太睏了") then
        if sm_state ~= "sleeping" then
            mud.emit("ac_exhausted")
        end
        return
    end
end)

-- ===== 定時循環 =====
function _G.AutoCast.loop(run_id)
    if run_id ~= _G.AutoCast.state.run_id then return end
    local sm_state = mud.sm_current("autocast")
    if not sm_state or sm_state == "stopped" then return end

    _G.AutoCast.state.check_count = _G.AutoCast.state.check_count + 1
    local next_delay = 3.0

    if sm_state == "initializing" then
        mud.send("score")
        next_delay = 2.5
    elseif sm_state == "sleeping" then
        if _G.AutoCast.state.check_count % 2 == 0 then
            mud.send("score")
        else
            mud.send("save")
        end
        next_delay = 20.0
    elseif sm_state == "casting" then
        if _G.AutoCast.state.check_count % 20 == 0 then
            mud.send("save")
            next_delay = 2.0
        else
            mud.send(_G.AutoCast.config.command)
            next_delay = 2.5
        end
    end

    if _G.AutoCast.config.debug then
        mud.echo(string.format("[Debug] Loop: state=%s, delay=%.1fs, count=%d",
            sm_state, next_delay, _G.AutoCast.state.check_count))
    end

    local rid = run_id
    mud.timer(next_delay, function() _G.AutoCast.loop(rid) end)
end

-- ===== 公開介面 =====
function _G.AutoCast.start(cmd)
    if cmd then _G.AutoCast.config.command = cmd end

    _G.AutoCast.state.run_id = _G.AutoCast.state.run_id + 1
    _G.AutoCast.state.check_count = 0

    mud.echo("🚀 AutoCast 啟動: " .. _G.AutoCast.config.command)
    MudUtils.start_log("autocast")

    mud.sm_transition("autocast", "ac_start")
    local rid = _G.AutoCast.state.run_id
    mud.timer(0.2, function() _G.AutoCast.loop(rid) end)
end

function _G.AutoCast.stop()
    _G.AutoCast.state.run_id = _G.AutoCast.state.run_id + 1
    mud.sm_transition("autocast", "ac_stop")
    MudUtils.stop_log()
end

function _G.AutoCast.status()
    local S = _G.AutoCast.state
    local sm_state = mud.sm_current("autocast") or "unknown"
    mud.echo("📊 AutoCast 狀態: " .. sm_state)
    mud.echo("   MP: " .. S.current_mp .. "/" .. S.max_mp)
    mud.echo("   指令: " .. _G.AutoCast.config.command)
end

function _G.AutoCast.reload()
    package.loaded["scripts.autocast"] = nil
    require("scripts.autocast")
    mud.echo("[AutoCast] ♻️ 腳本已重新載入")
end

-- ===== 載入訊息 =====
local usage = [[
指令:
  1. 啟動: /lua AutoCast.start("cast 'sum' boy")
  2. 停止: /lua AutoCast.stop()
  3. 狀態: /lua AutoCast.status()
  4. 重載: /lua AutoCast.reload()
說明:
  使用狀態機管理施法→睡覺→回魔循環。
  狀態可在 Debug 面板中即時查看。]]

mud.echo("========================================")
mud.echo("✅ AutoCast 自動練功腳本 (v2.0 事件驅動)")
mud.echo(usage)
mud.echo("========================================")

_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["AutoCast"] = {
    desc = "自動施法循環 (事件驅動版)",
    usage = usage
}
