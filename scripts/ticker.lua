-- ticker.lua
-- 這是一個通用的定期任務管理器
-- 使用方式: 
-- 1. 載入此腳本: /lua dofile("ticker.lua")
-- 2. 新增計時器: /lua Ticker.add("autoloot", 5, "get all from corpse")
-- 3. 停止計時器: /lua Ticker.stop("autoloot")
-- 4. 列出計時器: /lua Ticker.list()

_G.Ticker = {}
_G.Ticker.registry = {}

-- 新增計時器（不啟動）
-- name: 標識符 (String)
-- interval: 間隔秒數 (Number)
-- command: 要執行的指令 (String) 或 函數 (Function)
function _G.Ticker.add(name, interval, command)
    -- 如果已存在，先移除舊的
    _G.Ticker.remove(name)

    local entry = {
        name = name,
        interval = interval,
        command = command,
        enabled = false,
        count = 0
    }

    local cmd_desc = type(command) == "function" and "(Lua Function)" or tostring(command)
    _G.Ticker.registry[name] = entry
    mud.echo(string.format("✅ 計時器 '%s' 已新增: 每 %.1f 秒 [%s] (未啟動)", name, interval, cmd_desc))
end

-- 啟動計時器
function _G.Ticker.start(name)
    local entry = _G.Ticker.registry[name]
    if not entry then
        mud.echo(string.format("⚠️ 找不到計時器 '%s'", name))
        return
    end
    if entry.enabled then
        mud.echo(string.format("⚠️ 計時器 '%s' 已在運行中", name))
        return
    end
    entry.enabled = true
    entry.count = 0
    local cmd_desc = type(entry.command) == "function" and "(Lua Function)" or tostring(entry.command)
    mud.echo(string.format("▶️ 計時器 '%s' 已啟動: 每 %.1f 秒 [%s]", name, entry.interval, cmd_desc))
    mud.emit("ticker_started", {name = name, interval = entry.interval})
    _G.Ticker._schedule(name)
end

-- 停止計時器（保留定義）
function _G.Ticker.stop(name)
    local entry = _G.Ticker.registry[name]
    if not entry then
        mud.echo(string.format("⚠️ 找不到計時器 '%s'", name))
        return
    end
    if not entry.enabled then
        mud.echo(string.format("⚠️ 計時器 '%s' 未在運行", name))
        return
    end
    entry.enabled = false
    mud.echo(string.format("⏸️ 計時器 '%s' 已停止 (共執行 %d 次)", name, entry.count))
    mud.emit("ticker_stopped", {name = name, count = entry.count})
end

-- 移除計時器（完全刪除）
function _G.Ticker.remove(name)
    local entry = _G.Ticker.registry[name]
    if entry then
        entry.enabled = false
        _G.Ticker.registry[name] = nil
        mud.echo(string.format("🗑️ 計時器 '%s' 已移除", name))
    end
end

-- 內部：排程下一次執行
function _G.Ticker._schedule(name)
    local entry = _G.Ticker.registry[name]
    if entry and entry.enabled then
        local callback_code = string.format("_G.Ticker._execute('%s')", name)
        mud.timer(entry.interval, callback_code)
    end
end

-- 內部：執行並排程下一次
function _G.Ticker._execute(name)
    local entry = _G.Ticker.registry[name]
    if not entry or not entry.enabled then return end

    if type(entry.command) == "function" then
        local status, err = pcall(entry.command)
        if not status then
            mud.echo(string.format("⚠️ Ticker '%s' 執行錯誤: %s", name, err))
        end
    else
        for cmd in string.gmatch(entry.command, "[^;]+") do
            local clean_cmd = string.match(cmd, "^%s*(.-)%s*$")
            if clean_cmd and #clean_cmd > 0 then
                mud.send(clean_cmd)
            end
        end
    end
    entry.count = entry.count + 1
    _G.Ticker._schedule(name)
end

-- 停止所有計時器
function _G.Ticker.stop_all()
    for name, entry in pairs(_G.Ticker.registry) do
        if entry.enabled then
            entry.enabled = false
            mud.echo(string.format("⏸️ 計時器 '%s' 已停止 (共執行 %d 次)", name, entry.count))
        end
    end
end

-- 列出當前計時器
function _G.Ticker.list()
    mud.echo("=== 計時器列表 ===")
    local count = 0
    for name, entry in pairs(_G.Ticker.registry) do
        local status = entry.enabled and "▶️" or "⏸️"
        local cmd_desc = type(entry.command) == "function" and "(Fn)" or tostring(entry.command)
        mud.echo(string.format("  %s [%s] %.1fs : %s (已執行: %d)", status, name, entry.interval, cmd_desc, entry.count))
        count = count + 1
    end
    if count == 0 then
        mud.echo("  (無)")
    end
    mud.echo("==========================")
end

function _G.Ticker.reload()
    package.loaded["scripts.ticker"] = nil
    require("scripts.ticker")
    mud.echo("[Ticker] 腳本已重新載入")
end

local usage = [[
使用說明:
  1. 新增:   /lua Ticker.add('name', seconds, 'cmd')
  2. 啟動:   /lua Ticker.start('name')
  3. 停止:   /lua Ticker.stop('name')
  4. 移除:   /lua Ticker.remove('name')
  5. 列表:   /lua Ticker.list()
  6. 全停:   /lua Ticker.stop_all()
範例:
  /lua Ticker.add('heal', 5, 'cast cure light')
  /lua Ticker.start('heal')]]

mud.echo("========================================")
mud.echo("✅ Ticker 定時任務系統")
mud.echo(usage)
mud.echo("========================================")

-- Help 註冊
_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["Ticker"] = {
    desc = "定時任務管理器",
    usage = usage
}
