-- autocast.lua
-- 自動練功腳本 (耗盡 MP -> 睡覺 -> 滿 MP -> 喚醒 -> 繼續)
-- 載入: 自動
-- 使用: /lua AutoCast.start("cast 'summon' boy")
-- 停止: /lua AutoCast.stop()

_G.AutoCast = _G.AutoCast or {}

-- 設定
_G.AutoCast.config = {
    command = "cast 'summon' boy",
    debug = false
}

-- 狀態
_G.AutoCast.state = {
    mode = "stopped", -- "stopped", "initializing", "casting", "sleeping"
    max_mp = 0,
    current_mp = 0,
    run_id = 0,
    check_count = 0,
}

-- 訊息 Hook (每次載入都重新安裝)
-- 先保存非 AutoCast 的舊 hook
local base_hook = nil
if _G.on_server_message and not _G.AutoCast.hook_installed then
    base_hook = _G.on_server_message
elseif _G.AutoCast._base_hook then
    base_hook = _G.AutoCast._base_hook
end
_G.AutoCast._base_hook = base_hook

_G.on_server_message = function(line, clean_line)
    if base_hook then base_hook(line, clean_line) end
    if _G.AutoCast and _G.AutoCast.on_server_message then
        _G.AutoCast.on_server_message(line, clean_line)
    end
end
_G.AutoCast.hook_installed = true

function _G.AutoCast.on_server_message(line, clean_line)
    if _G.AutoCast.state.mode == "stopped" then return end

    -- 使用 clean_line 並進行 trim
    if not clean_line then return end
    local clean_line = string.match(clean_line, "^%s*(.-)%s*$")
    -- clean_line = string.gsub(clean_line, "\27%[[0-9;]*[mK]", "") -- 去除 ANSI (Rust 已處理)

    -- 1. 偵測耗盡訊息 (觸發睡覺)
    if string.find(clean_line, "耗盡") and string.find(clean_line, "精神力") then
        if _G.AutoCast.state.mode ~= "sleeping" then
            mud.echo("⚡ 精神力耗盡！準備休息...")
            _G.AutoCast.to_sleep()
        end
        return
    end

    -- 2. 偵測 Score 中的「精神力:   277/  578」(初始化或睡覺時使用)
    -- 超級寬鬆的正則，處理多餘空格
    if _G.AutoCast.state.mode == "sleeping" or _G.AutoCast.state.mode == "initializing" then
        local mp_score, max_mp_score = string.match(clean_line, "精神力[^%d]*(%d+)[^%d]*(%d+)")
        if mp_score and max_mp_score then
            local current = tonumber(mp_score)
            local total = tonumber(max_mp_score)
            _G.AutoCast.state.current_mp = current
            _G.AutoCast.state.max_mp = total
            
            if _G.AutoCast.config.debug then
                mud.echo(string.format("[Debug] Score Matched: MP=%d/%d, Mode=%s", current, total, _G.AutoCast.state.mode))
            end
            
            local percent = (total > 0) and (current / total * 100) or 0
            
            if _G.AutoCast.state.mode == "initializing" then
                -- 初始化完成，決定下一步
                if percent < 10 then
                    mud.echo(string.format("💤 MP 不足 (%.0f%%)，進入睡眠模式", percent))
                    _G.AutoCast.to_sleep()
                else
                    mud.echo(string.format("✅ MP 足夠 (%.0f%%)，開始施法", percent))
                    _G.AutoCast.state.mode = "casting"
                end
            elseif _G.AutoCast.state.mode == "sleeping" then
                -- 睡覺時檢查是否回滿
                if percent >= 98 then
                    mud.echo("🔋 MP 已回滿，起床繼續練功！")
                    _G.AutoCast.to_wake()
                end
            end
            return
        end
    end
    
    -- 3. "你太睏了" -> 確認睡覺
    if string.find(clean_line, "太睏了") then
        if _G.AutoCast.state.mode ~= "sleeping" then
             _G.AutoCast.state.mode = "sleeping"
             mud.echo("💤 確認進入睡眠狀態")
        end
        return
    end

    -- 4. 偵測狀態錯誤 (正在施法卻是睡覺狀態)
    if string.find(clean_line, "睡覺") then
        mud.echo("💤 偵測到睡眠關鍵字，嘗試喚醒...")
        mud.send("wake")
        return
    end
end

-- 動作轉換
function _G.AutoCast.to_sleep()
    if _G.AutoCast.state.mode == "sleeping" then return end
    _G.AutoCast.state.mode = "sleeping"
    mud.send("sleep")
end

function _G.AutoCast.to_wake()
    _G.AutoCast.state.mode = "casting"
    mud.send("wake")
end

-- 循環 Loop (核心驅動)
function _G.AutoCast.loop(run_id)
    if run_id ~= _G.AutoCast.state.run_id then return end
    if _G.AutoCast.state.mode == "stopped" then return end
    
    local next_delay = 3.0
    _G.AutoCast.state.check_count = (_G.AutoCast.state.check_count or 0) + 1

    if _G.AutoCast.state.mode == "initializing" then
        -- 初始化：發送 score 並立即進入施法模式
        -- 如果是睡眠狀態，後續的「你正在睡覺耶」觸發器會處理
        mud.send("score")
        _G.AutoCast.state.mode = "casting"
        mud.echo("✅ 初始化完成，開始施法")
        next_delay = 2.5
    elseif _G.AutoCast.state.mode == "sleeping" then
        -- 睡覺時：交替使用 score 與 save
        if _G.AutoCast.state.check_count % 2 == 0 then
            mud.send("score")
        else
            mud.send("save")
        end
        next_delay = 20.0 
    elseif _G.AutoCast.state.mode == "casting" then
        -- 施法時：每 20 次指令插入一次 save
        if _G.AutoCast.state.check_count % 20 == 0 then
            mud.send("save")
            next_delay = 2.0
        else
            mud.send(_G.AutoCast.config.command)
            next_delay = 2.5
        end
    end
    
    if _G.AutoCast.config.debug then
        mud.echo(string.format("[Debug] Loop: mode=%s, delay=%.1fs, count=%d", 
            _G.AutoCast.state.mode, next_delay, _G.AutoCast.state.check_count))
    end
    
    mud.timer(next_delay, string.format("_G.AutoCast.loop(%d)", run_id))
end

-- 公開介面
function _G.AutoCast.start(cmd)
    if cmd then _G.AutoCast.config.command = cmd end
    
    _G.AutoCast.state.run_id = _G.AutoCast.state.run_id + 1
    _G.AutoCast.state.mode = "initializing" -- 先檢查狀態
    _G.AutoCast.state.check_count = 0
    
    mud.echo("🚀 AutoCast 啟動: " .. _G.AutoCast.config.command)
    mud.echo("   正在檢查狀態...")
    
    _G.AutoCast.loop(_G.AutoCast.state.run_id)
end

function _G.AutoCast.stop()
    _G.AutoCast.state.mode = "stopped"
    _G.AutoCast.state.run_id = _G.AutoCast.state.run_id + 1
    mud.echo("🛑 AutoCast 已停止")
end

function _G.AutoCast.status()
    local S = _G.AutoCast.state
    mud.echo("📊 AutoCast 狀態: " .. S.mode)
    mud.echo("   MP: " .. S.current_mp .. "/" .. S.max_mp)
    mud.echo("   指令: " .. _G.AutoCast.config.command)
end

-- 註冊 Help
local usage = [[
指令:
  1. 啟動: /lua AutoCast.start("cast 'sum' boy")
  2. 停止: /lua AutoCast.stop()
  3. 狀態: /lua AutoCast.status()
說明:
  自動施法直到收到「耗盡精神力」訊息，
  然後自動睡覺 (sleep)，待 MP 回滿後
  自動喚醒 (wake) 並繼續施法。]]

mud.echo("========================================")
mud.echo("✅ AutoCast 自動練功腳本 已載入")
mud.echo(usage)
mud.echo("========================================")

_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["AutoCast"] = {
    desc = "自動施法循環 (含睡覺回魔)",
    usage = usage
}
