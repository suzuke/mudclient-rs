-- memcalc.lua
-- 技能/法術記憶點數計算機
-- 載入: /lua dofile("memcalc.lua")
-- 使用: /lua MemCalc.spell("holy arrow")

_G.MemCalc = _G.MemCalc or {}

-- 狀態
_G.MemCalc.state = {
    running = false,
    total_cost = 0,
    known_skills = {}, -- [name] = { cost=100, is_spell=false }
    pending_queue = {},
    current_query = nil,
    timeout_timer = nil,
    pending_prompts = 0,
    last_activity = 0,
    first_mode = nil, -- nil=both, "skill", "spell"
    is_first = true,
}

function _G.MemCalc.init()
    local usage = [[
使用說明:
  1. 查法術: /lua MemCalc.spell('name')
  2. 查技能: /lua MemCalc.skill('name')
  3. 自動查: /lua MemCalc.query('name')
範例:
  /lua MemCalc.spell('holy arrow')
  /lua MemCalc.skill('swordmaster')]]

    mud.echo("========================================")
    mud.echo("✅ MemCalc 記憶計算機 (v3.5 指定查詢版)")
    mud.echo(usage)
    mud.echo("========================================")
    
    -- 註冊到 Help 系統
    _G.Help = _G.Help or {}
    _G.Help.registry = _G.Help.registry or {}
    _G.Help.registry["MemCalc"] = {
        desc = "記憶點數計算機",
        usage = usage
    }
end

function _G.MemCalc.reset_timer()
    _G.MemCalc.state.last_activity = os.time()
end

-- 全域 Server Message Hook
function _G.on_server_message(line)
    if not _G.MemCalc or not _G.MemCalc.state.running then return end

    local clean_line = string.match(line, "^%s*(.-)%s*$")
    clean_line = string.gsub(clean_line, "\27%[[0-9;]*[mK]", "")

    -- Debug: 顯示原始行
    if string.find(clean_line, "記憶量") then
        mud.echo("Debug Hook: " .. clean_line)
    end
    
    -- 0. 判斷是否為法術 (依據: 花費法力)
    if string.find(clean_line, "花費法力") then
        local current = _G.MemCalc.state.current_query
        if current and _G.MemCalc.state.known_skills[current] then
            _G.MemCalc.state.known_skills[current].is_spell = true
            _G.MemCalc.reset_timer()
        end
    end

    -- 1. 抓取依賴技能
    if string.find(clean_line, "你需要學習") and string.find(clean_line, "記憶量") then
        local _, _, content = string.find(clean_line, "你需要學習[: ]%s*(.-)%s*記憶量")
        local _, _, cost = string.find(clean_line, "記憶量:%s*(%d+)")
        
        if content and cost then
            local eng_name = string.match(content, "^([%w%s]+)")
            if eng_name then
                local skill_name = string.match(eng_name, "^%s*(.-)%s*$")
                mud.echo("   -> Dep Found: [" .. skill_name .. "] Cost: " .. cost)
                
                if not _G.MemCalc.state.known_skills[skill_name] and not _G.MemCalc.in_queue(skill_name) then
                    table.insert(_G.MemCalc.state.pending_queue, skill_name)
                end
            end
        end
        return
    end

    -- 2. 抓取主技能
    if string.find(clean_line, "技能名稱") and string.find(clean_line, "記憶量") then
        local _, _, content = string.find(clean_line, "技能名稱%s*:%s*(.-)%s*記憶量")
        local _, _, cost = string.find(clean_line, "記憶量:%s*(%d+)")
        
        if content and cost then
            local eng_name = string.match(content, "^([%w%s]+)")
            local skill_name = ""
            
            if eng_name then
                skill_name = string.match(eng_name, "^%s*(.-)%s*$")
            end
            
            if skill_name == "" and _G.MemCalc.state.current_query then
                skill_name = _G.MemCalc.state.current_query
            end
            
            local cost_num = tonumber(cost)
            mud.echo("   -> Main Found: [" .. skill_name .. "] Cost: " .. cost_num)

            _G.MemCalc.reset_timer()

            if skill_name ~= "" and not _G.MemCalc.state.known_skills[skill_name] then
                _G.MemCalc.state.known_skills[skill_name] = { 
                    cost = cost_num, 
                    is_spell = false 
                }
                _G.MemCalc.state.total_cost = _G.MemCalc.state.total_cost + cost_num
                mud.echo(string.format("🔍 發現: %s (記憶: %d)", skill_name, cost_num))
            end
        end
        return
    end
    
    -- 3. 錯誤訊息 Gag
    if string.find(clean_line, "這不是一項技能喔") or 
       string.find(clean_line, "這不是一項法術喔") or
       string.find(clean_line, "沒有這種法術或技能") then
        mud.gag_message()
        return
    end

    -- 4. Prompt 偵測
    if string.match(clean_line, "^%s*%(%d+/%d+hp") then
        if _G.MemCalc.state.pending_prompts > 0 then
            _G.MemCalc.state.pending_prompts = _G.MemCalc.state.pending_prompts - 1
            if _G.MemCalc.state.pending_prompts <= 0 and _G.MemCalc.state.current_query then
                _G.MemCalc.state.current_query = nil
                _G.MemCalc.process_queue()
            end
        end
    end
end

function _G.MemCalc.in_queue(name)
    for _, v in ipairs(_G.MemCalc.state.pending_queue) do
        if v == name then return true end
    end
    return false
end

function _G.MemCalc.process_queue()
    if _G.MemCalc.state.current_query then return end

    if #_G.MemCalc.state.pending_queue == 0 then
        _G.MemCalc.finish()
        return
    end
    
    local next_skill = table.remove(_G.MemCalc.state.pending_queue, 1)
    
    if _G.MemCalc.state.known_skills[next_skill] then
        _G.MemCalc.process_queue()
        return
    end

    _G.MemCalc.state.current_query = next_skill
    _G.MemCalc.state.last_activity = os.time()
    
    -- 設定超時保護
    mud.timer(5.0, "_G.MemCalc.check_timeout('" .. next_skill .. "')")
    
    -- 決定查詢模式
    local mode = nil
    if _G.MemCalc.state.is_first then
        mode = _G.MemCalc.state.first_mode
        _G.MemCalc.state.is_first = false -- 之後的依賴一律用雙重查詢
    end

    if mode then
        -- 單一模式
        _G.MemCalc.state.pending_prompts = 1
        mud.timer(0.5, string.format("_G.MemCalc.send_queries('%s', '%s')", next_skill, mode))
    else
        -- 雙重模式
        _G.MemCalc.state.pending_prompts = 2
        mud.timer(0.5, string.format("_G.MemCalc.send_queries('%s', nil)", next_skill))
    end
end

function _G.MemCalc.check_timeout(skill_checking)
    if _G.MemCalc.state.current_query ~= skill_checking then return end
    
    local now = os.time()
    if (now - _G.MemCalc.state.last_activity) >= 4 then
        _G.MemCalc.force_next()
    else
        mud.timer(3.0, "_G.MemCalc.check_timeout('" .. skill_checking .. "')")
    end
end

function _G.MemCalc.send_queries(skill_name, mode)
    if mode == "spell" then
        mud.echo(">> 發送查詢 (Spell): " .. skill_name)
        mud.send("spell '" .. skill_name)
    elseif mode == "skill" then
        mud.echo(">> 發送查詢 (Skill): " .. skill_name)
        mud.send("skill '" .. skill_name)
    else
        mud.echo(">> 發送查詢 (Both): " .. skill_name)
        mud.send("spell '" .. skill_name)
        mud.send("skill '" .. skill_name)
    end
end

function _G.MemCalc.force_next()
    if _G.MemCalc.state.running and _G.MemCalc.state.current_query then
        mud.echo("⚠️ 查詢超時: " .. _G.MemCalc.state.current_query .. " (繼續下一項)")
        _G.MemCalc.state.current_query = nil
        _G.MemCalc.state.pending_prompts = 0
        _G.MemCalc.process_queue()
    end
end

function _G.MemCalc.start_scan(root_skill, mode)
    _G.MemCalc.state = {
        running = true,
        total_cost = 0,
        known_skills = {},
        pending_queue = {root_skill},
        current_query = nil,
        pending_prompts = 0,
        last_activity = os.time(),
        first_mode = mode, -- nil, "skill", "spell"
        is_first = true,
    }
    
    local type_str = mode and (mode == "spell" and "法術" or "技能") or "自動"
    mud.echo("🧮 開始計算 [" .. root_skill .. "] (" .. type_str .. ") 的總記憶需求...")
    _G.MemCalc.process_queue()
end

-- API
function _G.MemCalc.spell(name)
    _G.MemCalc.start_scan(name, "spell")
end

function _G.MemCalc.skill(name)
    _G.MemCalc.start_scan(name, "skill")
end

-- 相容舊版
function _G.MemCalc.query(name)
    _G.MemCalc.start_scan(name, nil)
end

function _G.MemCalc.finish()
    if not _G.MemCalc.state.running then return end
    _G.MemCalc.state.running = false
    
    mud.echo("--------------------------------------------------")
    mud.echo("📊 計算完成！")
    mud.echo("   總記憶點數需求: " .. _G.MemCalc.state.total_cost)
    mud.echo("   包含技能樹:")
    
    local sorted_skills = {}
    for name, data in pairs(_G.MemCalc.state.known_skills) do
        table.insert(sorted_skills, {name=name, cost=data.cost, is_spell=data.is_spell})
    end
    table.sort(sorted_skills, function(a,b) return a.cost > b.cost end)

    for _, s in ipairs(sorted_skills) do
        local type_str = s.is_spell and "[法術]" or "[技能]"
        mud.echo(string.format("   %s %-20s : %4d", type_str, s.name, s.cost))
    end
    mud.echo("--------------------------------------------------")
end

_G.MemCalc.init()
