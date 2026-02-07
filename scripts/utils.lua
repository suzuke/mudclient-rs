-- utils.lua
-- 通用工具函式庫
-- 載入方式: /lua dofile("utils.lua")

utils = {}

-- 1. 批量指令生成 (Loop Command)
-- 用於生成針對多個目標的指令，例如: 1.mob, 2.mob, 3.mob...
-- 用法: 
--   /lua utils.loop(3, "c sou %d.student")  -> "c sou 1.student", "c sou 2.student", "c sou 3.student"
--   /lua utils.loop(3, "kill %d.orc", true) -> "kill orc", "kill 2.orc", "kill 3.orc" (首個不帶數字)
--
-- 參數:
--   count: 執行次數
--   pattern: 指令樣板，%d 會被替換為數字
--   skip_first_num: (可選) 若為 true，則第一次執行時不加 "1."，直接使用 base name
function utils.loop(count, pattern, skip_first_num)
    for i = 1, count do
        local cmd = pattern
        
        if skip_first_num and i == 1 then
            -- 嘗試移除樣板中的 "%d." 或 "%d"
            cmd = string.gsub(cmd, "%%d%.", "") -- 移除 "1."
            cmd = string.gsub(cmd, "%%d", "")   -- 移除 "1"
        else
            cmd = string.gsub(cmd, "%%d", i)
        end
        
        mud.send(cmd)
    end
end

-- 2. 簡易別名註冊 (Lua Alias Helper)
-- 讓使用者更容易在腳本中定義別名
-- 用法: utils.alias("k", "kill target")
function utils.alias(name, command)
    -- 這需要客戶端支援動態新增 Alias 的 API，目前尚未實作
    -- 這裡僅示範，實際上您可以建立一個 Lua table 當作 alias 系統
    mud.echo("目前版本請使用客戶端內建別名管理器。")
end

-- 3. 延遲序列 (Sequence)
-- 依序執行一連串指令，每個指令間隔 delay 秒
function utils.sequence(delay, commands)
    for i, cmd in ipairs(commands) do
        local run_at = (i - 1) * delay
        if run_at == 0 then
            mud.send(cmd)
        else
            -- 這裡需要 escaping 引號
            local safe_cmd = string.gsub(cmd, "'", "\\'")
            mud.timer(run_at, string.format("mud.send('%s')", safe_cmd))
        end
    end
end

local usage = [[
使用說明:
  1. 批量指令: /lua utils.loop(count, 'cmd %d')
  2. 延遲序列: /lua utils.sequence(delay, {'cmd1', 'cmd2'})
範例:
  /lua utils.loop(5, 'get all from %d.corpse')]]

mud.echo("========================================")
mud.echo("✅ Utils 通用工具庫")
mud.echo(usage)
mud.echo("========================================")

-- Help 註冊
_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["Utils"] = {
    desc = "通用工具庫",
    usage = usage
}
