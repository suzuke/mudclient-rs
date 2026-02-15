-- ticker.lua
-- 這是一個通用的定期任務管理器
-- 使用方式: 
-- 1. 載入此腳本: /lua dofile("ticker.lua")
-- 2. 新增計時器: /lua Ticker.add("autoloot", 5, "get all from corpse")
-- 3. 停止計時器: /lua Ticker.stop("autoloot")
-- 4. 列出計時器: /lua Ticker.list()

_G.Ticker = {}
_G.Ticker.registry = {}

-- 新增或更新計時器
-- name: 標識符 (String)
-- interval: 間隔秒數 (Number)
-- command: 要執行的指令 (String) 或 函數 (Function)
function _G.Ticker.add(name, interval, command)
    -- 先停止舊的以免重複
    _G.Ticker.stop(name)
    
    local entry = {
        name = name,
        interval = interval,
        command = command,
        enabled = true,
        count = 0
    }
    
    local cmd_desc = type(command) == "function" and "(Lua Function)" or tostring(command)
    _G.Ticker.registry[name] = entry
    
    mud.echo(string.format("✅ 計時器 '%s' 已啟動: 每 %.1f 秒執行一次 [%s]", name, interval, cmd_desc))
    
    -- 啟動遞迴回調
    _G.Ticker.callback(name)
end

-- 內部回調函數
function _G.Ticker.callback(name)
    local entry = _G.Ticker.registry[name]
    
    -- 檢查是否存在且啟用
    if entry and entry.enabled then
        if type(entry.command) == "function" then
            -- 如果是函數，直接執行 (主要用於 utils.loop 等複雜操作)
            -- 使用 pcall 保護避免錯誤中斷 ticker
            local status, err = pcall(entry.command)
            if not status then
                mud.echo(string.format("⚠️ Ticker '%s' 執行錯誤: %s", name, err))
            end
        else
            -- 支援多指令分號拆分
            for cmd in string.gmatch(entry.command, "[^;]+") do
                local clean_cmd = string.match(cmd, "^%s*(.-)%s*$")
                if clean_cmd and #clean_cmd > 0 then
                    mud.send(clean_cmd)
                end
            end
        end
        entry.count = entry.count + 1
        
        -- 設定下一次執行 (建構回調字串)
        local callback_code = string.format("_G.Ticker.callback('%s')", name)
        mud.timer(entry.interval, callback_code)
    end
end

-- 停止計時器
function _G.Ticker.stop(name)
    local entry = _G.Ticker.registry[name]
    if entry then
        entry.enabled = false
        _G.Ticker.registry[name] = nil
        mud.echo(string.format("🛑 計時器 '%s' 已停止 (共執行 %d 次)", name, entry.count))
    else
        mud.echo(string.format("⚠️ 找不到計時器 '%s'", name))
    end
end

-- 停止所有計時器
function _G.Ticker.stop_all()
    for name, _ in pairs(_G.Ticker.registry) do
        _G.Ticker.stop(name)
    end
    mud.echo("全部計時器已停止。")
end

-- 列出當前計時器 status
function _G.Ticker.list()
    mud.echo("=== 活躍計時器列表 ===")
    local count = 0
    for name, entry in pairs(_G.Ticker.registry) do
        if entry.enabled then
            mud.echo(string.format("  [%s] %.1fs : %s (已執行: %d)", name, entry.interval, entry.command, entry.count))
            count = count + 1
        end
    end
    if count == 0 then
        mud.echo("  (無)")
    end
    mud.echo("==========================")
end

function _G.Ticker.reload()
    package.loaded["scripts.ticker"] = nil
    require("scripts.ticker")
    mud.echo("[Ticker] ♻️ 腳本已重新載入")
end

local usage = [[
使用說明:
  1. 新增: /lua Ticker.add('name', seconds, 'cmd')
  2. 停止: /lua Ticker.stop('name')
  3. 列表: /lua Ticker.list()
範例:
  /lua Ticker.add('heal', 5, 'cast cure light')]]

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
