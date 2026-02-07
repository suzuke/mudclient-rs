-- combat.lua
-- 戰鬥輔助腳本
-- 載入: /lua dofile("combat.lua")

_G.Combat = _G.Combat or {}

-- 預設參數
_G.Combat.default_target = "student"
_G.Combat.default_count = 10 -- 預設群攻數量

-- 1. 群體施法 (Mass Cast)
-- 用法: /lua Combat.mass_cast("soulsteal", "student", 5)
-- 結果:
--   cast 'soulsteal' student
--   cast 'soulsteal' 2.student
--   ...
--   cast 'soulsteal' 5.student
function _G.Combat.mass_cast(spell, target, count)
    target = target or _G.Combat.default_target
    count = count or _G.Combat.default_count
    
    mud.echo(string.format("⚔️ 發動群體施法: %s x%d 對像 %s", spell, count, target))
    
    local cmds = {}
    for i = 1, count do
        local t = target
        if i > 1 then
            t = i .. "." .. target
        end
        table.insert(cmds, string.format("cast '%s' %s", spell, t))
    end
    
    -- 一次性發送所有指令 (若 MUD 支援緩衝)
    -- 若需要延遲，可改用 utils.sequence
    for _, cmd in ipairs(cmds) do
        mud.send(cmd)
    end
end

-- 2. 特化：吸魂 (Soulsteal)
-- 用法: /lua Combat.soulsteal(10) 或 /lua Combat.soulsteal()
function _G.Combat.soulsteal(count, target)
    target = target or "student"
    -- 如果沒給 count，嘗試從 args 推斷，或用預設
    count = count or 10
    
    _G.Combat.mass_cast("soulsteal", target, count)
end

-- 初始化顯示
function _G.Combat.init()
    local usage = [[
功能:
  1. 群體施法: Combat.mass_cast(spell, target, n)
  2. 快速吸魂: Combat.soulsteal(n, [target])
範例:
  /lua Combat.soulsteal(8)
  /lua Combat.mass_cast('magic missile', 'orc', 3)]]

    mud.echo("========================================")
    mud.echo("⚔️ Combat 戰鬥輔助系統")
    mud.echo(usage)
    mud.echo("========================================")
    
    -- Help 註冊
    _G.Help = _G.Help or {}
    _G.Help.registry = _G.Help.registry or {}
    _G.Help.registry["Combat"] = {
        desc = "戰鬥輔助 (群體施法)",
        usage = usage
    }
end

_G.Combat.init()
