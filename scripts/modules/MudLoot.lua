-- MudLoot Module
-- 提供通用、智慧型的搜刮邏輯
-- 使用 mud.collect_response 由 Rust 底層偵測 prompt 作為回應結束標記
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
    callback = nil,
    options = {},
    rid = 0,
}

-- ===== 核心 API =====

--- 開始搜刮流程
--- @param options table { items: string[], sac: bool, loot_ground: bool, fallback_blind: bool }
--- @param callback function 結束後的回呼
function MudLoot.process_loot(options, callback)
    local s = MudLoot.state
    s.options = options or {}
    s.callback = callback
    s.rid = MudUtils.run_id

    if MudLoot.config.debug then mud.echo("[MudLoot] 🔍 使用 collect_response 掃描環境...") end
    mud.collect_response("l", "_G.MudLoot._on_scan_collected()")
end

--- collect_response 回呼：解析完整的 look 輸出
function MudLoot._on_scan_collected()
    local s = MudLoot.state
    if s.rid ~= MudUtils.run_id then return end

    local lines = _G._collected_lines or {}
    local found_corpses = 0
    local found_items = {}
    local in_container = false
    local opt = s.options

    for _, line in ipairs(lines) do
        -- 容器內容跳過
        if string.find(line, "裡面有:", 1, true) or string.find(line, "contains:", 1, true) then
            in_container = true
        elseif line == "" or string.find(line, "^%(ID: ") then
            in_container = false
        end

        if not in_container then
            -- 解析屍體
            if string.find(line, "的屍體", 1, true) or string.find(line, "/corpse", 1, true) then
                local count_match = line:match("^%s*%(%s*(%d+)%)")
                if count_match then
                    local count = tonumber(count_match)
                    if count and count > found_corpses then
                        found_corpses = count
                    end
                else
                    found_corpses = found_corpses + 1
                end
            end

            -- 解析目標物品 (地面)
            if opt.loot_ground and opt.items then
                local stripped = line:gsub("^[%s%(%d%)]*", ""):gsub("^%s*[(.-)]%s*", "")
                for _, target in ipairs(opt.items) do
                    if string.find(stripped, target, 1, true) then
                        table.insert(found_items, { name = target, is_ground = true })
                        if MudLoot.config.debug then mud.echo("[MudLoot] 📦 發現地面物品: " .. target) end
                        break
                    end
                end
            end
        end
    end

    if MudLoot.config.debug then
        mud.echo("[MudLoot] 📊 掃描結果: " .. found_corpses .. " 具屍體, " .. #found_items .. " 個地面物品")
    end

    MudLoot.execute_actions(found_corpses, found_items)
end

-- ===== 執行行動 =====

function MudLoot.execute_actions(found_corpses, found_items)
    local s = MudLoot.state
    local opt = s.options
    local cmds = {}

    -- 1. 處理地面物品
    for _, item in ipairs(found_items) do
        table.insert(cmds, "get " .. item.name)
    end

    -- 2. 處理屍體
    if found_corpses > 0 then
        for i = 1, found_corpses do
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

_G.MudLoot = MudLoot
return MudLoot
