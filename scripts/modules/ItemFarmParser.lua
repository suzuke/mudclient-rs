-- scripts/modules/ItemFarmParser.lua
-- Centralized MUD text -> semantic event parser for ItemFarm v3

local string = string
local tonumber = tonumber
local ipairs = ipairs

local M = {}

-- Static parse rules: { pattern, event, [parse_fn] }
-- parse_fn(clean_line) -> data table or nil
local RULES = {
    -- Combat events
    { p = "魂歸西天了",             ev = "ifarm:mob_killed" },
    { p = "逃了",                  ev = "ifarm:mob_fled" },
    { p = "離開了",                ev = "ifarm:mob_fled" },
    { p = "目標不在",              ev = "ifarm:target_missing" },
    { p = "施法的目標不在",         ev = "ifarm:target_missing" },
    { p = "沒有這個生物",           ev = "ifarm:target_missing" },

    -- Search events
    { p = "他正在這個世界中",        ev = "ifarm:search_found", data = {type = "quest"} },
    { p = "攜帶著",                ev = "ifarm:search_found", data = {type = "locate"} },

    -- Charm events
    { p = "開始跟隨你了",           ev = "ifarm:charm_success" },
    { p = "不受你的言語所迷惑",      ev = "ifarm:charm_resist" },

    -- Combat detection (emergency)
    { p = "伺機而動",              ev = "ifarm:unexpected_combat" },
    { p = "蓄勢待發",              ev = "ifarm:unexpected_combat" },
    { p = "身陷戰鬥中",            ev = "ifarm:unexpected_combat" },

    -- Flee
    { p = "你為了保命而不顧面子從戰鬥中逃了", ev = "ifarm:flee_success" },
    { p = "你逃跑失敗了",          ev = "ifarm:flee_failed" },

    -- Loot
    { p = "丟下了",                ev = "ifarm:mob_dropped" },

    -- Spell list
    { p = "目前對你產生影響的法術或技巧有", ev = "ifarm:spell_list_start" },
}

-- Dynamic rules added at runtime (buff fade messages)
local dynamic_rules = {}

function M.add_fade_rule(fade_msg, indicator)
    dynamic_rules[#dynamic_rules + 1] = {
        p = fade_msg,
        ev = "ifarm:buff_faded",
        data = { indicator = indicator },
    }
end

function M.clear_dynamic_rules()
    dynamic_rules = {}
end

-- Parse "你報告自己的狀況: HP/Max 生命力 MA/Max 精神力 V/Max 移動力 P/Max 內力"
local function parse_report(line)
    local hp, hp_max = string.match(line, "(%d+)/(%d+)%s+生命力")
    local mp, mp_max = string.match(line, "(%d+)/(%d+)%s+精神力")
    if hp and mp then
        return {
            hp = tonumber(hp), hp_max = tonumber(hp_max),
            mp = tonumber(mp), mp_max = tonumber(mp_max),
        }
    end
    return nil
end

-- Parse "法術: 'XXX' ... 達 N 小時"
local function parse_spell(line)
    local name, hours = string.match(line, "法術:%s+'(.-)'.*達%s+(-?%d+)%s+小時")
    if name then
        return { name = name, hours = tonumber(hours) }
    end
    return nil
end

-- Main parse function: called from on_server_message hook
-- Returns nothing; emits events via mud.emit()
function M.parse(line, clean_line, is_echo)
    if is_echo then return end

    local cl = clean_line:gsub("\r", "")
    local len = #cl
    if len < 2 then return end

    -- Skip chat/emote lines
    if string.find(cl, "^【") then return end
    if string.find(cl, "^%s*「.*」") then return end

    -- Status report (special: has parse function)
    if string.find(cl, "你報告自己的狀況", 1, true) then
        local data = parse_report(cl)
        if data then mud.emit("ifarm:status_report", data) end
        return
    end

    -- Spell detection
    local spell_data = parse_spell(cl)
    if spell_data then
        mud.emit("ifarm:spell_detected", spell_data)
        return
    end

    -- Score-based HP/MP parsing (fallback for score command output)
    local h_cur, h_max = string.match(cl, "生命力:?%s+(%d+)/%s+(%d+)")
    if h_cur then
        local m_cur, m_max = string.match(cl, "精神力:?%s+(%d+)/%s+(%d+)")
        mud.emit("ifarm:score_hp_mp", {
            hp = tonumber(h_cur), hp_max = tonumber(h_max),
            mp = m_cur and tonumber(m_cur) or nil,
            mp_max = m_max and tonumber(m_max) or nil,
        })
        return
    end

    -- Static rules
    for _, rule in ipairs(RULES) do
        if string.find(cl, rule.p, 1, true) then
            local data = rule.data or {}
            data.line = cl
            mud.emit(rule.ev, data)
            return  -- first match wins
        end
    end

    -- Dynamic rules (buff fade)
    for _, rule in ipairs(dynamic_rules) do
        if string.find(cl, rule.p, 1, true) then
            mud.emit(rule.ev, rule.data)
            return
        end
    end
end

return M
