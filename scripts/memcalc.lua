-- memcalc.lua
-- 技能/法術記憶點數計算機
-- 載入: /lua dofile("memcalc.lua")
-- 使用: /lua MemCalc.spell("holy arrow")

_G.MemCalc = _G.MemCalc or {}

-- 狀態
_G.MemCalc.state = {
    running = false,
    total_cost = 0,
    known_skills = {}, -- [name] = { cost, is_spell, dependencies={}, exclusions={} }
    pending_queue = {},
    current_query = nil,
    pending_deps = {},  -- 當前正在查詢的技能的相依列表
    pending_excl = {},  -- 當前正在查詢的技能的相斥列表
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
  3. 快速掃描: /lua MemCalc.scan_all()
  4. 完整掃描: /lua MemCalc.scan_full()
     (含相依性，耗時較長)
  5. 停止掃描: /lua MemCalc.stop_scan()
  6. 儲存資料: /lua MemCalc.save()]]

    mud.echo("========================================")
    mud.echo("✅ MemCalc 記憶計算機 (v5.0 完整掃描版)")
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

-- 掃描狀態
_G.MemCalc.scan_state = {
    scanning = false,
    current_type = nil, -- "spell" or "skill"
    scanned_data = {},  -- {name = {cost, is_spell}}
    spell_count = 0,
    skill_count = 0,
}

-- 掃描所有技能和法術
function _G.MemCalc.scan_all()
    _G.MemCalc.scan_state = {
        scanning = true,
        current_type = "spell",
        scanned_data = {},
        spell_count = 0,
        skill_count = 0,
    }
    
    mud.echo("🔍 開始掃描所有法術和技能...")
    mud.echo("   (請等待掃描完成，可能需要按 Enter 翻頁)")
    
    -- 先掃描法術
    mud.send("spell all")
    
    -- 5 秒後切換到技能掃描
    mud.timer(5.0, "_G.MemCalc.scan_skills()")
end

function _G.MemCalc.scan_skills()
    _G.MemCalc.scan_state.current_type = "skill"
    mud.send("skill all")
    
    -- 10 秒後完成掃描
    mud.timer(10.0, "_G.MemCalc.scan_finish()")
end

function _G.MemCalc.scan_finish()
    if not _G.MemCalc.scan_state.scanning then return end
    _G.MemCalc.scan_state.scanning = false
    
    local data = _G.MemCalc.scan_state.scanned_data
    local spell_count = _G.MemCalc.scan_state.spell_count
    local skill_count = _G.MemCalc.scan_state.skill_count
    
    -- 合併到 known_skills (用於 save 功能)
    _G.MemCalc.state.known_skills = data
    
    mud.echo("--------------------------------------------------")
    mud.echo("📊 掃描完成！")
    mud.echo(string.format("   法術: %d 項, 技能: %d 項, 總計: %d 項", 
        spell_count, skill_count, spell_count + skill_count))
    
    -- 如果是完整掃描模式，繼續查詢相依性
    if _G.MemCalc.scan_state.full_mode then
        _G.MemCalc.start_dep_scan()
    else
        mud.echo("💡 使用 MemCalc.save() 將資料儲存到資料庫")
        mud.echo("--------------------------------------------------")
    end
end

-- 完整掃描模式狀態
_G.MemCalc.full_scan = {
    running = false,
    queue = {},      -- 待查詢的技能列表
    current = nil,   -- 當前正在查詢的技能
    completed = 0,   -- 已完成數量
    total = 0,       -- 總數量
}

-- 開始完整掃描（含相依性）
function _G.MemCalc.scan_full()
    _G.MemCalc.scan_state = {
        scanning = true,
        current_type = "spell",
        scanned_data = {},
        spell_count = 0,
        skill_count = 0,
        full_mode = true,  -- 標記為完整掃描模式
    }
    
    _G.MemCalc.full_scan = {
        running = true,
        queue = {},
        current = nil,
        completed = 0,
        total = 0,
    }
    
    mud.echo("🔍 開始完整掃描（含相依性）...")
    mud.echo("   第一階段: 蒐集所有技能列表")
    mud.echo("   (請等待，過程中請按 Enter 翻頁)")
    
    -- 先掃描法術
    mud.send("spell all")
    
    -- 5 秒後切換到技能掃描
    mud.timer(5.0, "_G.MemCalc.scan_skills()")
end

-- 開始相依性掃描
function _G.MemCalc.start_dep_scan()
    local data = _G.MemCalc.scan_state.scanned_data
    
    -- 建立待查詢隊列
    local queue = {}
    for name, skill_data in pairs(data) do
        table.insert(queue, {name = name, is_spell = skill_data.is_spell})
    end
    
    _G.MemCalc.full_scan.queue = queue
    _G.MemCalc.full_scan.total = #queue
    _G.MemCalc.full_scan.completed = 0
    
    mud.echo("")
    mud.echo("   第二階段: 查詢各技能相依性")
    mud.echo(string.format("   共 %d 項技能待查詢，預估需要 %d 分鐘", #queue, math.ceil(#queue * 2 / 60)))
    mud.echo("--------------------------------------------------")
    
    -- 開始查詢
    _G.MemCalc.query_next_dep()
end

-- 查詢下一個技能的相依性
function _G.MemCalc.query_next_dep()
    if not _G.MemCalc.full_scan.running then return end
    
    local queue = _G.MemCalc.full_scan.queue
    
    -- 先儲存上一個技能的相依/相斥 (在清空之前)
    if _G.MemCalc.state.current_query then
        local prev_skill = _G.MemCalc.state.current_query
        if _G.MemCalc.state.known_skills[prev_skill] then
            -- 只有在有資料時才更新
            if #_G.MemCalc.state.pending_deps > 0 then
                _G.MemCalc.state.known_skills[prev_skill].dependencies = _G.MemCalc.state.pending_deps
            end
            if #_G.MemCalc.state.pending_excl > 0 then
                _G.MemCalc.state.known_skills[prev_skill].exclusions = _G.MemCalc.state.pending_excl
            end
        end
    end
    
    if #queue == 0 then
        _G.MemCalc.finish_full_scan()
        return
    end
    
    local next_item = table.remove(queue, 1)
    _G.MemCalc.full_scan.current = next_item.name
    _G.MemCalc.full_scan.completed = _G.MemCalc.full_scan.completed + 1
    
    -- 清空 pending 列表 (準備收集新技能的資料)
    _G.MemCalc.state.pending_deps = {}
    _G.MemCalc.state.pending_excl = {}
    _G.MemCalc.state.current_query = next_item.name
    _G.MemCalc.state.running = true
    
    -- 進度顯示（每 10 個顯示一次）
    if _G.MemCalc.full_scan.completed % 10 == 1 then
        mud.echo(string.format("⏳ 進度: %d/%d (%.0f%%)", 
            _G.MemCalc.full_scan.completed, 
            _G.MemCalc.full_scan.total,
            _G.MemCalc.full_scan.completed / _G.MemCalc.full_scan.total * 100))
    end
    
    -- 發送查詢
    if next_item.is_spell then
        mud.send("spell '" .. next_item.name)
    else
        mud.send("skill '" .. next_item.name)
    end
    
    -- 2 秒後查詢下一個
    mud.timer(2.0, "_G.MemCalc.query_next_dep()")
end

-- 完成完整掃描
function _G.MemCalc.finish_full_scan()
    _G.MemCalc.full_scan.running = false
    _G.MemCalc.state.running = false
    
    -- 統計有相依的技能數量
    local with_deps = 0
    for name, data in pairs(_G.MemCalc.state.known_skills) do
        if data.dependencies and #data.dependencies > 0 then
            with_deps = with_deps + 1
        end
    end
    
    mud.echo("--------------------------------------------------")
    mud.echo("🎉 完整掃描完成！")
    mud.echo(string.format("   總計: %d 項技能", _G.MemCalc.full_scan.total))
    mud.echo(string.format("   有相依資料: %d 項", with_deps))
    mud.echo("")
    
    -- 自動儲存
    _G.MemCalc.save()
end

-- 停止完整掃描
function _G.MemCalc.stop_scan()
    if _G.MemCalc.full_scan.running then
        _G.MemCalc.full_scan.running = false
        _G.MemCalc.state.running = false
        mud.echo("🛑 掃描已停止")
        mud.echo("💡 使用 MemCalc.save() 可儲存已收集的資料")
    elseif _G.MemCalc.scan_state.scanning then
        _G.MemCalc.scan_state.scanning = false
        mud.echo("🛑 掃描已停止")
    else
        mud.echo("⚠️ 目前沒有正在進行的掃描")
    end
end

function _G.MemCalc.reset_timer()
    _G.MemCalc.state.last_activity = os.time()
end

-- 輔助函數：檢查陣列是否包含某值
function _G.MemCalc.array_contains(arr, val)
    for _, v in ipairs(arr) do
        if v == val then return true end
    end
    return false
end

-- 全域 Server Message Hook (使用鏈接模式)
-- 確保與 Practice 等其他腳本共存
if not _G.MemCalc.hook_installed then
    local old_hook = _G.on_server_message
    _G.on_server_message = function(line, clean_line)
        -- 先執行舊的 hook (例如 Practice)
        if old_hook then old_hook(line, clean_line) end
        -- 再執行 MemCalc 的處理
        if _G.MemCalc and _G.MemCalc.on_server_message then
            _G.MemCalc.on_server_message(line, clean_line)
        end
    end
    _G.MemCalc.hook_installed = true
end

function _G.MemCalc.on_server_message(line, clean_line)
    -- local clean_line = string.match(line, "^%s*(.-)%s*$") -- 這裡先不 match，保留原始空白結構，或使用 Rust 傳來的版本
    --Rust 傳來的 clean_line 已經去除了 ANSI code，但不保證 trim。
    -- 原本邏輯有 trim: string.match(line, "^%s*(.-)%s*$")
    -- 我們這裡簡單 trim 一下 clean_line 即可，或者直接用。
    
    -- 為了相容原本邏輯 (Match ^%s*(.-)%s*$)，我們對 clean_line 做一次 trim
    if not clean_line then return end
    local clean_line = string.match(clean_line, "^%s*(.-)%s*$")
    -- clean_line = string.gsub(clean_line, "\27%[[0-9;]*[mK]", "") -- 已由 Rust 處理
    
    -- 掃描模式解析
    if _G.MemCalc.scan_state.scanning then
        -- 解析格式: "            spell_name           中文名 記憶量: cost"
        -- 匹配: 英文名 + 任意字元 + "記憶量:" + 數字
        local eng_name, cost = string.match(clean_line, "^%s*([%w%s]+)%s+.+記憶量:%s*(%d+)")
        
        if eng_name and cost then
            -- 清理英文名 (去除首尾空白)
            eng_name = string.match(eng_name, "^%s*(.-)%s*$")
            
            if eng_name ~= "" then
                local cost_num = tonumber(cost)
                local is_spell = (_G.MemCalc.scan_state.current_type == "spell")
                
                _G.MemCalc.scan_state.scanned_data[eng_name] = {
                    cost = cost_num,
                    is_spell = is_spell
                }
                
                if is_spell then
                    _G.MemCalc.scan_state.spell_count = _G.MemCalc.scan_state.spell_count + 1
                else
                    _G.MemCalc.scan_state.skill_count = _G.MemCalc.scan_state.skill_count + 1
                end
            end
        end
        return
    end
    
    -- 單項查詢邏輯 (需要 running 狀態)
    if not _G.MemCalc.state.running then return end
    
    -- 0. 判斷是否為法術 (依據: 花費法力)
    if string.find(clean_line, "花費法力") then
        local current = _G.MemCalc.state.current_query
        if current and _G.MemCalc.state.known_skills[current] then
            _G.MemCalc.state.known_skills[current].is_spell = true
            _G.MemCalc.reset_timer()
        end
    end

    -- 1. 抓取依賴技能 (你需要學習)
    if string.find(clean_line, "你需要學習") and string.find(clean_line, "記憶量") then
        local _, _, content = string.find(clean_line, "你需要學習[: ]%s*(.-)%s*記憶量")
        local _, _, cost = string.find(clean_line, "記憶量:%s*(%d+)")
        
        if content and cost then
            local eng_name = string.match(content, "^([%w%s]+)")
            if eng_name then
                local skill_name = string.match(eng_name, "^%s*(.-)%s*$")
                mud.echo("   -> Dep Found: [" .. skill_name .. "] Cost: " .. cost)
                
                -- 儲存相依關係 (去重)
                if _G.MemCalc.state.current_query then
                    if not _G.MemCalc.array_contains(_G.MemCalc.state.pending_deps, skill_name) then
                        table.insert(_G.MemCalc.state.pending_deps, skill_name)
                    end
                end
                
                if not _G.MemCalc.state.known_skills[skill_name] and not _G.MemCalc.in_queue(skill_name) then
                    table.insert(_G.MemCalc.state.pending_queue, skill_name)
                end
            end
        end
        return
    end

    -- 1.5 抓取相斥技能 (你不能學習)
    if string.find(clean_line, "你不能學習") and string.find(clean_line, "記憶量") then
        local _, _, content = string.find(clean_line, "你不能學習[: ]%s*(.-)%s*記憶量")
        
        if content then
            local eng_name = string.match(content, "^([%w%s]+)")
            if eng_name then
                local skill_name = string.match(eng_name, "^%s*(.-)%s*$")
                mud.echo("   -> Excl Found: [" .. skill_name .. "] (相斥)")
                
                -- 儲存相斥關係 (去重)
                if _G.MemCalc.state.current_query then
                    if not _G.MemCalc.array_contains(_G.MemCalc.state.pending_excl, skill_name) then
                        table.insert(_G.MemCalc.state.pending_excl, skill_name)
                    end
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
            
            -- 在 scan_full 模式下：
            -- - 主技能行先出現，此時 pending_deps/excl 包含的是 *上一個* 技能的資料
            -- - 所以這裡只建立/更新基本資訊，不存入 deps/excl
            -- - deps/excl 會在下一輪 query_next_dep() 開始時存入
            
            if skill_name ~= "" then
                if _G.MemCalc.state.known_skills[skill_name] then
                    -- 更新現有技能的基本資訊 (不動 deps/excl)
                    _G.MemCalc.state.known_skills[skill_name].cost = cost_num
                    mud.echo(string.format("   ℹ️ 技能已存在: %s", skill_name))
                else
                    -- 新增技能 (空的 deps/excl，之後會填入)
                    _G.MemCalc.state.known_skills[skill_name] = { 
                        cost = cost_num, 
                        is_spell = false,
                        dependencies = {},
                        exclusions = {}
                    }
                    _G.MemCalc.state.total_cost = _G.MemCalc.state.total_cost + cost_num
                    mud.echo(string.format("🔍 發現: %s (記憶: %d)", skill_name, cost_num))
                end
                
                -- 清空 pending 列表，準備收集這個技能的 deps/excl
                _G.MemCalc.state.pending_deps = {}
                _G.MemCalc.state.pending_excl = {}
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
    mud.echo("💡 使用 MemCalc.save() 可將資料儲存到資料庫")
end

-- 資料庫路徑
_G.MemCalc.DB_PATH = "data/skills_db.json"

-- 簡易 JSON 編碼（支援 dependencies 與 exclusions 陣列）
function _G.MemCalc.json_encode(tbl)
    local parts = {"{"}
    local first = true
    for name, data in pairs(tbl) do
        if not first then table.insert(parts, ",") end
        first = false
        
        -- 編碼 dependencies 陣列
        local deps_str = "[]"
        if data.dependencies and #data.dependencies > 0 then
            local dep_parts = {}
            for _, dep in ipairs(data.dependencies) do
                table.insert(dep_parts, '"' .. dep .. '"')
            end
            deps_str = "[" .. table.concat(dep_parts, ", ") .. "]"
        end
        
        -- 編碼 exclusions 陣列
        local excl_str = "[]"
        if data.exclusions and #data.exclusions > 0 then
            local excl_parts = {}
            for _, ex in ipairs(data.exclusions) do
                table.insert(excl_parts, '"' .. ex .. '"')
            end
            excl_str = "[" .. table.concat(excl_parts, ", ") .. "]"
        end
        
        table.insert(parts, string.format('\n  "%s": {"cost": %d, "is_spell": %s, "dependencies": %s, "exclusions": %s}',
            name, data.cost, data.is_spell and "true" or "false", deps_str, excl_str))
    end
    table.insert(parts, "\n}")
    return table.concat(parts)
end

-- 簡易 JSON 解碼（支援 dependencies 與 exclusions 陣列）
function _G.MemCalc.json_decode(str)
    local result = {}
    
    -- 使用逐行解析
    for line in string.gmatch(str, '[^\n]+') do
        local name = string.match(line, '"([^"]+)":%s*{')
        if name then
            local cost = string.match(line, '"cost":%s*(%d+)')
            local is_spell = string.match(line, '"is_spell":%s*(%w+)')
            local deps_str = string.match(line, '"dependencies":%s*%[([^%]]*)%]')
            local excl_str = string.match(line, '"exclusions":%s*%[([^%]]*)%]')
            
            if cost then
                local dependencies = {}
                if deps_str and deps_str ~= "" then
                    for dep in string.gmatch(deps_str, '"([^"]+)"') do
                        table.insert(dependencies, dep)
                    end
                end
                
                local exclusions = {}
                if excl_str and excl_str ~= "" then
                    for ex in string.gmatch(excl_str, '"([^"]+)"') do
                        table.insert(exclusions, ex)
                    end
                end
                
                result[name] = {
                    cost = tonumber(cost),
                    is_spell = (is_spell == "true"),
                    dependencies = dependencies,
                    exclusions = exclusions
                }
            end
        end
    end
    
    -- 向後相容：舊格式沒有 dependencies/exclusions
    if next(result) == nil then
        for name, cost, is_spell in string.gmatch(str, '"([^"]+)":%s*{%s*"cost":%s*(%d+),%s*"is_spell":%s*(%w+)%s*}') do
            result[name] = {
                cost = tonumber(cost),
                is_spell = (is_spell == "true"),
                dependencies = {},
                exclusions = {}
            }
        end
    end
    
    return result
end

-- 載入資料庫
function _G.MemCalc.load_db()
    local file = io.open(_G.MemCalc.DB_PATH, "r")
    if not file then
        return {}
    end
    local content = file:read("*all")
    file:close()
    return _G.MemCalc.json_decode(content)
end

-- 儲存資料庫
function _G.MemCalc.save_db(db)
    local file = io.open(_G.MemCalc.DB_PATH, "w")
    if not file then
        mud.echo("❌ 無法寫入資料庫檔案: " .. _G.MemCalc.DB_PATH)
        return false
    end
    file:write(_G.MemCalc.json_encode(db))
    file:close()
    return true
end

-- 匯出當前查詢結果
function _G.MemCalc.export()
    if not _G.MemCalc.state.known_skills or next(_G.MemCalc.state.known_skills) == nil then
        mud.echo("⚠️ 沒有可匯出的資料，請先執行 MemCalc.spell() 或 MemCalc.skill()")
        return nil
    end
    local json = _G.MemCalc.json_encode(_G.MemCalc.state.known_skills)
    mud.echo("📋 匯出的 JSON 資料:")
    mud.echo(json)
    return _G.MemCalc.state.known_skills
end

-- 儲存到資料庫（合併模式）
function _G.MemCalc.save()
    if not _G.MemCalc.state.known_skills or next(_G.MemCalc.state.known_skills) == nil then
        mud.echo("⚠️ 沒有可儲存的資料")
        return false
    end
    
    -- 載入現有資料庫
    local db = _G.MemCalc.load_db()
    local new_count = 0
    local update_count = 0
    
    -- 合併新資料
    for name, data in pairs(_G.MemCalc.state.known_skills) do
        if not db[name] then
            db[name] = data
            new_count = new_count + 1
        else
            -- 更新現有資料
            db[name] = data
            update_count = update_count + 1
        end
    end
    
    -- 儲存
    if _G.MemCalc.save_db(db) then
        local total = 0
        for _ in pairs(db) do total = total + 1 end
        mud.echo(string.format("✅ 資料庫已更新: 新增 %d 項, 更新 %d 項, 總計 %d 項", new_count, update_count, total))
        return true
    end
    return false
end

-- 查看資料庫狀態
function _G.MemCalc.db_status()
    local db = _G.MemCalc.load_db()
    local count = 0
    local spell_count = 0
    local skill_count = 0
    
    for name, data in pairs(db) do
        count = count + 1
        if data.is_spell then
            spell_count = spell_count + 1
        else
            skill_count = skill_count + 1
        end
    end
    
    mud.echo("📊 技能資料庫狀態:")
    mud.echo(string.format("   總計: %d 項 (法術: %d, 技能: %d)", count, spell_count, skill_count))
    mud.echo("   路徑: " .. _G.MemCalc.DB_PATH)
end

_G.MemCalc.init()

