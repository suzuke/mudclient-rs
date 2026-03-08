-- practice.lua
-- 法術練習腳本 (v3.0 事件驅動版)
-- 使用 mud.collect_response 取代 hook 掃描
-- 載入: /lua dofile("practice.lua")
-- 啟動: /lua Practice.start()
-- 掃描: /lua Practice.scan()

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("Practice cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

-- 移除舊版 hook（如有殘留）
MudUtils.unregister_hook("Practice")

_G.Practice = _G.Practice or {}

-- ===== 設定區 =====
_G.Practice.spells = _G.Practice.spells or {}
_G.Practice.spell_info = _G.Practice.spell_info or {}

_G.Practice.target = "student"
_G.Practice.targetobject = "life"
_G.Practice.interval = 5.0
_G.Practice.soulsteal_count = 7

-- 狀態旗標
_G.Practice.running = false
_G.Practice.index = 1

-- 特殊指令覆蓋表
_G.Practice.special_cmds = {
    ["ventriloquate"] = "cast 'ventriloquate' someone hit me!",
}

-- 已知法術類型（解決 help 指令衝突的問題）
_G.Practice.known_spell_types = {
    ["sleep"] = "target",
    ["soulsteal"] = "target",
}

-- ===== 掃描邏輯 (使用 collect_response) =====

function _G.Practice.scan()
    mud.echo("🔍 開始自動掃描未滿 99% 的技能...")
    _G.Practice.spells = {}
    _G.Practice.spell_info = {}
    _G.Practice.index = 1

    mud.collect_response("pra", function(lines)
        _G.Practice._on_pra_collected(lines)
    end)
end

function _G.Practice._on_pra_collected(lines)
    lines = lines or {}
    local candidates = {}

    for _, line in ipairs(lines) do
        for name, prof in string.gmatch(line, "%[%s*(.-)%s*%]熟練度:%s*(%d+)") do
            if name and prof then
                local p = tonumber(prof)
                if p < 99 then
                    table.insert(candidates, name)
                    mud.echo("   收到候選: " .. name .. " (" .. p .. "%)")
                end
            end
        end
    end

    mud.echo("📋 列表掃描完成，開始分析 " .. #candidates .. " 個技能...")
    _G.Practice._candidates = candidates
    _G.Practice._check_next_candidate()
end

function _G.Practice._check_next_candidate()
    local candidates = _G.Practice._candidates
    if not candidates or #candidates == 0 then
        _G.Practice._finish_scan()
        return
    end

    local next_skill = table.remove(candidates, 1)

    -- 已知類型，跳過 help 查詢
    if _G.Practice.known_spell_types[next_skill] then
        local spell_type = _G.Practice.known_spell_types[next_skill]
        local exists = false
        for _, s in ipairs(_G.Practice.spells) do
            if s == next_skill then exists = true; break end
        end
        if not exists then
            table.insert(_G.Practice.spells, next_skill)
            _G.Practice.spell_info[next_skill] = { type = spell_type }
            mud.echo("✅ 加入法術: " .. next_skill .. " (類型: " .. spell_type .. ") [已知]")
        end
        _G.Practice._check_next_candidate()
        return
    end

    -- 查詢 help 以判斷法術類型
    local skill_name = next_skill
    mud.collect_response("help " .. next_skill, function(lines)
        _G.Practice._on_help_collected(skill_name, lines)
    end)
end

function _G.Practice._on_help_collected(skill_name, lines)
    lines = lines or {}

    for _, line in ipairs(lines) do
        if string.find(line, "格式") and string.find(line, "cast") then
            local info = { type = "self" }
            if string.find(line, "<character>") or string.find(line, "victim") then
                info.type = "target"
            elseif string.find(line, "<object>") or string.find(line, "item") then
                info.type = "object"
            end

            local exists = false
            for _, s in ipairs(_G.Practice.spells) do
                if s == skill_name then exists = true; break end
            end
            if not exists then
                table.insert(_G.Practice.spells, skill_name)
                _G.Practice.spell_info[skill_name] = info
                mud.echo("✅ 加入法術: " .. skill_name .. " (類型: " .. info.type .. ")")
            end
            break
        end
    end

    -- 繼續下一個
    _G.Practice._check_next_candidate()
end

function _G.Practice._finish_scan()
    _G.Practice._candidates = nil
    mud.echo("🎉 掃描完成！共找到 " .. #_G.Practice.spells .. " 個可練習法術。")
    mud.echo("輸入 /lua Practice.start() 開始練習。")
    _G.Practice.status()
end

-- ===== 核心函數 =====

local function send_cmd(cmd)
    for part in string.gmatch(cmd, "[^;]+") do
        local clean = string.match(part, "^%s*(.-)%s*$")
        if clean and #clean > 0 then
            mud.send(clean)
        end
    end
end

local function build_cmd(spell)
    local P = _G.Practice

    if P.special_cmds[spell] then return P.special_cmds[spell] end

    if spell == "soulsteal" then
        local t = {}
        for i = 1, P.soulsteal_count do
            local target = (i == 1) and P.target or (i .. "." .. P.target)
            table.insert(t, "cast 'soulsteal' " .. target)
        end
        return table.concat(t, "; ")
    end

    local info = P.spell_info[spell]
    local spell_type = info and info.type or "self"

    if spell_type == "target" then
        return "cast '" .. spell .. "' " .. P.target
    elseif spell_type == "object" then
        return "cast '" .. spell .. "' " .. P.targetobject
    else
        return "cast '" .. spell .. "'"
    end
end

local function get_spell_delay(spell)
    if spell == "soulsteal" then
        return _G.Practice.soulsteal_count * 2.0
    end
    return _G.Practice.interval
end

-- 執行循環
function _G.Practice.loop()
    if not _G.Practice.running then return end

    if #_G.Practice.spells == 0 then
        mud.echo("⚠️ 練習列表為空，請先使用 /lua Practice.scan() 或 Practice.add()")
        _G.Practice.running = false
        return
    end

    local spell = _G.Practice.spells[_G.Practice.index]
    if not spell then
        _G.Practice.index = 1
        spell = _G.Practice.spells[1]
    end

    mud.echo(string.format("🔮 [%d/%d] %s (預計耗時 %.1fs)",
        _G.Practice.index, #_G.Practice.spells, spell, get_spell_delay(spell)))
    send_cmd(build_cmd(spell))

    local delay = get_spell_delay(spell)
    _G.Practice.index = _G.Practice.index + 1
    if _G.Practice.index > #_G.Practice.spells then
        _G.Practice.index = 1
    end

    if _G.Practice.running then
        mud.timer(delay, function() _G.Practice.loop() end)
    end
end

-- 管理功能
function _G.Practice.start(target)
    if target then _G.Practice.target = target end
    _G.Practice.running = true
    mud.echo("🎓 開始練習... (目標: " .. _G.Practice.target .. ")")
    _G.Practice.loop()
end

function _G.Practice.stop()
    _G.Practice.running = false
    mud.echo("🛑 練習已停止")
end

function _G.Practice.add(spell)
    table.insert(_G.Practice.spells, spell)
    _G.Practice.spell_info[spell] = { type = "self" }
    mud.echo("✅ 已新增: " .. spell)
end

function _G.Practice.remove(spell)
    for i, s in ipairs(_G.Practice.spells) do
        if s == spell then
            table.remove(_G.Practice.spells, i)
            mud.echo("🗑️ 已移除: " .. spell)
            if i < _G.Practice.index then _G.Practice.index = _G.Practice.index - 1 end
            return
        end
    end
    mud.echo("⚠️ 找不到: " .. spell)
end

function _G.Practice.clear()
    _G.Practice.spells = {}
    _G.Practice.spell_info = {}
    _G.Practice.index = 1
    mud.echo("🧹 列表已清空")
end

function _G.Practice.status()
    mud.echo("📊 練習狀態:")
    mud.echo("   目標: " .. _G.Practice.target .. " | 物品: " .. _G.Practice.targetobject)
    mud.echo("   列表: " .. #_G.Practice.spells .. " 個法術")

    for i, s in ipairs(_G.Practice.spells) do
        local info = _G.Practice.spell_info[s]
        local t = info and info.type or "?"
        local mark = (i == _G.Practice.index) and ">" or " "
        mud.echo(string.format("   %s %d. %-15s [%s]", mark, i, s, t))
    end
end

function _G.Practice.reload()
    package.loaded["scripts.practice"] = nil
    require("scripts.practice")
    mud.echo("[Practice] ♻️ 腳本已重新載入")
end

-- 初始化顯示
local usage = [[
指令:
  1. 自動掃描: /lua Practice.scan()
  2. 開始練習: /lua Practice.start('target')
  3. 停止練習: /lua Practice.stop()
  4. 查看狀態: /lua Practice.status()
  5. 重載腳本: /lua Practice.reload()]]

mud.echo("========================================")
mud.echo("✅ Practice 自動練習腳本 (v3.0 collect_response)")
mud.echo(usage)
mud.echo("========================================")

_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["Practice"] = {
    desc = "自動練習腳本",
    usage = usage
}
