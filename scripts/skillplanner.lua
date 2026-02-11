-- skillplanner.lua
-- 技能配置規劃工具 (含相依性支援)
-- 載入: 自動
-- 使用: /lua SkillPlanner.add("holy arrow")

_G.SkillPlanner = _G.SkillPlanner or {}

-- 狀態
_G.SkillPlanner.state = {
    budget = 100,        -- 記憶點數上限
    selected = {},       -- 已選擇的技能 {name = true}
    db = {},             -- 技能資料庫
}

-- 資料庫路徑 (與 MemCalc 共用)
_G.SkillPlanner.DB_PATH = "data/skills_db.json"

-- JSON 解碼 (支援 dependencies)
function _G.SkillPlanner.json_decode(str)
    local result = {}
    
    -- 使用逐行解析
    for line in string.gmatch(str, '[^\n]+') do
        local name = string.match(line, '"([^"]+)":%s*{')
        if name then
            local cost = string.match(line, '"cost":%s*(%d+)')
            local is_spell = string.match(line, '"is_spell":%s*(%w+)')
            local deps_str = string.match(line, '"dependencies":%s*%[([^%]]*)%]')
            
            if cost then
                local dependencies = {}
                if deps_str and deps_str ~= "" then
                    for dep in string.gmatch(deps_str, '"([^"]+)"') do
                        table.insert(dependencies, dep)
                    end
                end
                
                -- 解析 exclusions
                local excl_str = string.match(line, '"exclusions":%s*%[([^%]]*)%]')
                local exclusions = {}
                if excl_str and excl_str ~= "" then
                    for ex in string.gmatch(excl_str, '"([^"]+)"') do
                        table.insert(exclusions, ex)
                    end
                end
                
                result[name] = {
                    cost = tonumber(cost),
                    is_spell = (is_spell == "true"),
                    dependencies = dependencies or {},
                    exclusions = exclusions or {}
                }
            end
        end
    end
    
    -- 向後相容舊格式
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
function _G.SkillPlanner.load()
    local file = io.open(_G.SkillPlanner.DB_PATH, "r")
    if not file then
        mud.echo("⚠️ 資料庫檔案不存在，請先用 MemCalc 收集資料")
        return false
    end
    local content = file:read("*all")
    file:close()
    
    _G.SkillPlanner.state.db = _G.SkillPlanner.json_decode(content)
    
    local count = 0
    local with_deps = 0
    for name, data in pairs(_G.SkillPlanner.state.db) do 
        count = count + 1 
        if data.dependencies and #data.dependencies > 0 then
            with_deps = with_deps + 1
        end
    end
    mud.echo(string.format("✅ 已載入技能資料庫: %d 項 (含相依資料: %d 項)", count, with_deps))
    return true
end

-- 設定記憶點數上限
function _G.SkillPlanner.budget(points)
    _G.SkillPlanner.state.budget = points
    mud.echo("💰 記憶點數上限設為: " .. points)
end

-- 遞迴取得所有相依技能
function _G.SkillPlanner.get_all_dependencies(name, visited)
    visited = visited or {}
    if visited[name] then return {} end
    visited[name] = true
    
    local db = _G.SkillPlanner.state.db
    local result = {}
    
    if db[name] and db[name].dependencies then
        for _, dep in ipairs(db[name].dependencies) do
            if not visited[dep] then
                table.insert(result, dep)
                -- 遞迴取得相依的相依
                local sub_deps = _G.SkillPlanner.get_all_dependencies(dep, visited)
                for _, sub_dep in ipairs(sub_deps) do
                    table.insert(result, sub_dep)
                end
            end
        end
    end
    
    return result
end

-- 加入技能 (自動加入相依, 檢查相斥)
function _G.SkillPlanner.add(name, quiet)
    local db = _G.SkillPlanner.state.db
    
    if not db[name] then
        mud.echo("⚠️ 找不到技能: " .. name)
        mud.echo("   請先用 MemCalc.spell('" .. name .. "') 收集資料")
        return false
    end
    
    -- 檢查相斥衝突
    local conflicts = {}
    
    -- 1. 檢查要加的技能是否與已選的技能相斥
    if db[name].exclusions then
        for _, excl in ipairs(db[name].exclusions) do
            if _G.SkillPlanner.state.selected[excl] then
                table.insert(conflicts, {skill = excl, reason = name .. " 與 " .. excl .. " 相斥"})
            end
        end
    end
    
    -- 2. 檢查已選的技能是否排斥這個技能
    for sel_name, _ in pairs(_G.SkillPlanner.state.selected) do
        if db[sel_name] and db[sel_name].exclusions then
            for _, excl in ipairs(db[sel_name].exclusions) do
                if excl == name then
                    table.insert(conflicts, {skill = sel_name, reason = sel_name .. " 與 " .. name .. " 相斥"})
                end
            end
        end
    end
    
    if #conflicts > 0 then
        mud.echo("❌ 無法加入 " .. name .. "，有相斥衝突:")
        for _, c in ipairs(conflicts) do
            mud.echo("   ⚔️ " .. c.reason)
        end
        return false
    end
    
    local added = {}
    
    -- 先加入相依技能
    local deps = _G.SkillPlanner.get_all_dependencies(name)
    for _, dep in ipairs(deps) do
        if not _G.SkillPlanner.state.selected[dep] then
            if db[dep] then
                _G.SkillPlanner.state.selected[dep] = true
                table.insert(added, dep)
            end
        end
    end
    
    -- 再加入主技能
    if not _G.SkillPlanner.state.selected[name] then
        _G.SkillPlanner.state.selected[name] = true
        table.insert(added, name)
    end
    
    if not quiet then
        if #added > 1 then
            mud.echo("✅ 加入技能: " .. name)
            mud.echo("   📎 自動加入相依技能:")
            for i, dep in ipairs(added) do
                if dep ~= name then
                    local cost = db[dep] and db[dep].cost or 0
                    mud.echo(string.format("      - %s (%d)", dep, cost))
                end
            end
        elseif #added == 1 then
            mud.echo("✅ 加入技能: " .. name)
        else
            mud.echo("ℹ️ 技能已在選擇中: " .. name)
        end
    end
    
    return true
end

-- 移除技能 (檢查是否為其他技能的相依)
function _G.SkillPlanner.remove(name)
    if not _G.SkillPlanner.state.selected[name] then
        mud.echo("⚠️ 未選擇此技能: " .. name)
        return false
    end
    
    local db = _G.SkillPlanner.state.db
    local dependents = {}
    
    -- 檢查是否有其他選中的技能依賴這個技能
    for sel_name, _ in pairs(_G.SkillPlanner.state.selected) do
        if sel_name ~= name and db[sel_name] and db[sel_name].dependencies then
            for _, dep in ipairs(db[sel_name].dependencies) do
                if dep == name then
                    table.insert(dependents, sel_name)
                    break
                end
            end
        end
    end
    
    if #dependents > 0 then
        mud.echo("⚠️ 無法移除 " .. name .. "，以下技能需要它:")
        for _, dep_name in ipairs(dependents) do
            mud.echo("   - " .. dep_name)
        end
        return false
    end
    
    _G.SkillPlanner.state.selected[name] = nil
    mud.echo("🗑️ 移除技能: " .. name)
    return true
end

-- 清空選擇
function _G.SkillPlanner.clear()
    _G.SkillPlanner.state.selected = {}
    mud.echo("🗑️ 已清空所有選擇")
end

-- 計算配置
function _G.SkillPlanner.plan()
    local db = _G.SkillPlanner.state.db
    local selected = _G.SkillPlanner.state.selected
    local budget = _G.SkillPlanner.state.budget
    
    if next(selected) == nil then
        mud.echo("⚠️ 尚未選擇任何技能")
        return
    end
    
    local total = 0
    local skills = {}
    
    for name, _ in pairs(selected) do
        if db[name] then
            local dep_count = db[name].dependencies and #db[name].dependencies or 0
            table.insert(skills, {
                name = name, 
                cost = db[name].cost, 
                is_spell = db[name].is_spell,
                dep_count = dep_count
            })
            total = total + db[name].cost
        end
    end
    
    table.sort(skills, function(a, b) return a.cost > b.cost end)
    
    mud.echo("--------------------------------------------------")
    mud.echo("📊 技能配置規劃結果")
    mud.echo("")
    
    for _, s in ipairs(skills) do
        local type_str = s.is_spell and "[法術]" or "[技能]"
        local dep_str = s.dep_count > 0 and string.format(" (需%d項)", s.dep_count) or ""
        mud.echo(string.format("   %s %-20s : %4d%s", type_str, s.name, s.cost, dep_str))
    end
    
    mud.echo("")
    mud.echo(string.format("   總記憶點數: %d / %d", total, budget))
    
    if total > budget then
        mud.echo("   ❌ 超出預算 " .. (total - budget) .. " 點！")
    else
        mud.echo("   ✅ 剩餘空間 " .. (budget - total) .. " 點")
    end
    mud.echo("--------------------------------------------------")
end

-- 建議可加入的技能 (考慮相依成本)
function _G.SkillPlanner.suggest()
    local db = _G.SkillPlanner.state.db
    local selected = _G.SkillPlanner.state.selected
    local budget = _G.SkillPlanner.state.budget
    
    -- 計算已用點數
    local used = 0
    for name, _ in pairs(selected) do
        if db[name] then
            used = used + db[name].cost
        end
    end
    
    local remaining = budget - used
    
    if remaining <= 0 then
        mud.echo("⚠️ 預算已用盡")
        return
    end
    
    -- 找出可加入的技能 (計算含相依的總成本)
    local suggestions = {}
    for name, data in pairs(db) do
        if not selected[name] then
            -- 計算總成本 (技能本身 + 未選擇的相依)
            local total_cost = data.cost
            local deps = _G.SkillPlanner.get_all_dependencies(name)
            for _, dep in ipairs(deps) do
                if not selected[dep] and db[dep] then
                    total_cost = total_cost + db[dep].cost
                end
            end
            
            if total_cost <= remaining then
                table.insert(suggestions, {
                    name = name, 
                    cost = data.cost, 
                    total_cost = total_cost,
                    is_spell = data.is_spell,
                    deps_needed = #deps
                })
            end
        end
    end
    
    table.sort(suggestions, function(a, b) return a.total_cost > b.total_cost end)
    
    mud.echo("--------------------------------------------------")
    mud.echo("💡 可加入的技能 (剩餘 " .. remaining .. " 點):")
    mud.echo("")
    
    local count = 0
    for _, s in ipairs(suggestions) do
        if count >= 10 then
            mud.echo("   ... 還有更多 ...")
            break
        end
        local type_str = s.is_spell and "[法術]" or "[技能]"
        local dep_str = ""
        if s.deps_needed > 0 and s.total_cost > s.cost then
            dep_str = string.format(" (含相依共 %d)", s.total_cost)
        end
        mud.echo(string.format("   %s %-20s : %4d%s", type_str, s.name, s.cost, dep_str))
        count = count + 1
    end
    
    if count == 0 then
        mud.echo("   (沒有符合預算的技能)")
    end
    mud.echo("--------------------------------------------------")
end

-- 列出所有已知技能
function _G.SkillPlanner.list()
    local db = _G.SkillPlanner.state.db
    
    local skills = {}
    for name, data in pairs(db) do
        table.insert(skills, {name = name, cost = data.cost, is_spell = data.is_spell})
    end
    
    table.sort(skills, function(a, b) return a.cost > b.cost end)
    
    mud.echo("--------------------------------------------------")
    mud.echo("📚 技能資料庫:")
    mud.echo("")
    
    for _, s in ipairs(skills) do
        local type_str = s.is_spell and "[法術]" or "[技能]"
        local sel = _G.SkillPlanner.state.selected[s.name] and " ★" or ""
        mud.echo(string.format("   %s %-20s : %4d%s", type_str, s.name, s.cost, sel))
    end
    mud.echo("--------------------------------------------------")
end

-- 顯示技能的相依樹
function _G.SkillPlanner.deps(name)
    local db = _G.SkillPlanner.state.db
    
    if not db[name] then
        mud.echo("⚠️ 找不到技能: " .. name)
        return
    end
    
    local data = db[name]
    local type_str = data.is_spell and "[法術]" or "[技能]"
    
    mud.echo("--------------------------------------------------")
    mud.echo(string.format("📎 %s %s 的相依性:", type_str, name))
    mud.echo(string.format("   記憶點數: %d", data.cost))
    
    if data.dependencies and #data.dependencies > 0 then
        mud.echo("   需要學習:")
        local total_dep_cost = 0
        for _, dep in ipairs(data.dependencies) do
            local dep_cost = db[dep] and db[dep].cost or 0
            total_dep_cost = total_dep_cost + dep_cost
            mud.echo(string.format("      - %s (%d)", dep, dep_cost))
        end
        mud.echo(string.format("   相依總成本: %d", total_dep_cost))
        mud.echo(string.format("   完整成本: %d", data.cost + total_dep_cost))
    else
        mud.echo("   無相依技能")
    end
    mud.echo("--------------------------------------------------")
end

-- 初始化
function _G.SkillPlanner.init()
    local usage = [[
使用說明:
  1. 載入資料: /lua SkillPlanner.load()
  2. 設定上限: /lua SkillPlanner.budget(85)
  3. 加入技能: /lua SkillPlanner.add('holy arrow')
     (自動加入相依技能)
  4. 查看配置: /lua SkillPlanner.plan()
  5. 建議技能: /lua SkillPlanner.suggest()
  6. 相依查詢: /lua SkillPlanner.deps('holy arrow')
  7. 移除技能: /lua SkillPlanner.remove('fireball')
  8. 清空選擇: /lua SkillPlanner.clear()
  9. 列出全部: /lua SkillPlanner.list()]]

    mud.echo("========================================")
    mud.echo("✅ SkillPlanner 技能配置規劃工具 (v2.0 相依版)")
    mud.echo(usage)
    mud.echo("========================================")
    
    -- 自動載入資料庫
    _G.SkillPlanner.load()
    
    -- 註冊到 Help 系統
    _G.Help = _G.Help or {}
    _G.Help.registry = _G.Help.registry or {}
    _G.Help.registry["SkillPlanner"] = {
        desc = "技能配置規劃工具 (含相依性)",
        usage = usage
    }
end

_G.SkillPlanner.init()
