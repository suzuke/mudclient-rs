-- MudLoot Module
-- 提供通用、智慧型的搜刮邏輯
-- 載入方式: local MudLoot = require("scripts.modules.MudLoot")

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("MudLoot cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

local MudLoot = {}

-- 狀態
MudLoot.state = {
    scanning = false,
    in_container = false,
    found_corpses = 0,
    found_items = {}, -- { {name = "xxx", is_ground = true/false} }
    callback = nil,
    options = {},
    rid = 0
}

-- ===== 核心 API =====

--- 開始搜刮流程
--- @param options table { items: string[], sac: bool, loot_ground: bool, fallback_blind: bool, max_corpses: int }
--- @param callback function 結束後的回呼
function MudLoot.process_loot(options, callback)
    local s = MudLoot.state
    s.options = options or {}
    s.callback = callback
    s.rid = MudUtils.run_id
    
    -- 重置解析狀態
    s.scanning = true
    s.in_container = false
    s.found_corpses = 0
    s.found_items = {}
    
    if MudLoot.config.debug and mud then mud.echo("[MudLoot] 🔍 開始掃描環境...") end
    mud.send("l")
    
    -- 設定保底計時器，避免解析失敗卡住
    if not _G.MudUtils then
        -- 無 MudUtils 時直接執行，不等待
        MudLoot.execute_actions()
        return
    end
    
    MudUtils.safe_timer(1.5, function(rid)
        if rid == s.rid and s.scanning then
            if MudLoot.config.debug then mud.echo("[MudLoot] ⚠️ 掃描超時，執行保底行動") end
            s.scanning = false
            MudLoot.execute_actions()
        end
    end)
end

-- ===== 解析邏輯 (State Machine) =====

function MudLoot.on_server_message(line, clean_line)
    local s = MudLoot.state
    if not s.scanning then return end
    clean_line = clean_line or line or ""
    
    -- 1. 偵測容器/提示符以切換狀態
    if string.find(clean_line, "裡面有:", 1, true) or string.find(clean_line, "contains:", 1, true) then
        s.in_container = true
        return
    end
    
    -- 提示符或空行通常代表看完了 (根據不同 MUD 可能需要調整)
    if clean_line == "" or string.find(clean_line, "^%(ID: ") then
        s.in_container = false
    end
    
    -- 2. 只有在此次 "look" 的輸出中且不在容器內才解析
    if s.in_container then return end
    
    -- 3. 解析屍體
    if string.find(clean_line, "的屍體", 1, true) or string.find(clean_line, "/corpse", 1, true) then
        -- Check if there's a multiplier like "( 2) 賈不妙 那慘不忍睹的屍體/corpse"
        local count_match = clean_line:match("^%s*%(%s*(%d+)%)")
        if count_match then
            local count = tonumber(count_match)
            if count and count > s.found_corpses then
                s.found_corpses = count
            end
        else
            s.found_corpses = s.found_corpses + 1
        end
        return
    end
    
    -- 4. 解析目標物品 (地面)
    if s.options.loot_ground and s.options.items then
        -- 剝除前綴 (數字、狀態括號)
        local stripped = clean_line:gsub("^[%s%(%d%)]*", ""):gsub("^%s*[(.-)]%s*", "")
        
        for _, target in ipairs(s.options.items) do
            if string.find(stripped, target, 1, true) then
                table.insert(s.found_items, { name = target, is_ground = true })
                if MudLoot.config.debug then mud.echo("[MudLoot] 📦 發現地面物品: " .. target) end
                break
            end
        end
    end
    
    -- 5. 結束偵測 (看到出口表示描述結束)
    if string.find(clean_line, "[出口:", 1, true) then
        s.scanning = false
        MudUtils.safe_timer(0.2, MudLoot.execute_actions)
    end
end

-- ===== 執行行動 =====

function MudLoot.execute_actions()
    local s = MudLoot.state
    local opt = s.options
    local cmds = {}
    
    -- 1. 處理地面物品
    for _, item in ipairs(s.found_items) do
        table.insert(cmds, "get " .. item.name)
    end
    
    -- 2. 處理屍體
    if s.found_corpses > 0 then
        for i = 1, s.found_corpses do
            local corpse_ref = (i == 1) and "corpse" or (i .. ".corpse")
            for _, item in ipairs(opt.items or {"all"}) do
                table.insert(cmds, "get " .. item .. " " .. corpse_ref)
            end
            if opt.sac then
                table.insert(cmds, "sac " .. corpse_ref)
            end
        end
    elseif opt.fallback_blind then
        -- 盲抓保底
        for _, item in ipairs(opt.items or {"all"}) do
            table.insert(cmds, "get " .. item .. " corpse")
        end
        if opt.sac then
            table.insert(cmds, "sac corpse")
        end
    end
    
    -- 3. 發送指令
    if #cmds > 0 then
        if MudLoot.config.debug then mud.echo("[MudLoot] 🚀 執行搜刮指令 (" .. #cmds .. " 條)") end
        for _, cmd in ipairs(cmds) do
            mud.send(cmd)
        end
    else
        if MudLoot.config.debug then mud.echo("[MudLoot] 💨 無可搜刮目標") end
    end
    
    -- 4. 回呼
    if s.callback then
        local delay = #cmds * 0.1 + 0.5
        MudUtils.safe_timer(delay, function(rid)
            if rid == s.rid and s.callback then
                s.callback()
            end
        end)
    end
end

-- ===== 設定與初始化 =====

MudLoot.config = {
    debug = true
}

-- ===== 註冊到 Hook Registry =====
MudUtils.register_hook("MudLoot", function(line, clean_line)
    MudLoot.on_server_message(line, clean_line)
end)

return MudLoot
