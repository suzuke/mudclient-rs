-- practice.lua
-- 法術練習腳本
-- 載入: /lua dofile("practice.lua")
-- 啟動: /lua Practice.start()
-- 掃描: /lua Practice.scan()

_G.Practice = _G.Practice or {}

-- ===== 設定區 =====
-- 這些列表保留作為 "最後手段" 或預設值，但主要依賴掃描結果
_G.Practice.spells = _G.Practice.spells or {}
_G.Practice.spell_info = _G.Practice.spell_info or {} -- 動態儲存法術資訊 { type="target"|"object"|"self" }

_G.Practice.target = "student"       -- 預設目標 (生物)
_G.Practice.targetobject = "life"     -- 預設物品 (用於 identify/locate 等)
_G.Practice.interval = 5.0           -- 基本施法間隔 (秒)
_G.Practice.soulsteal_count = 7      -- soulsteal 連發次數

-- 狀態旗標
_G.Practice.running = false
_G.Practice.index = 1

-- 掃描狀態
_G.Practice.scan_state = {
    active = false,
    stage = nil, -- "parsing_pra", "checking_help"
    candidates = {}, -- 待檢查的技能名稱
    current_check = nil,
    timeout_timer = nil,
    pending_prompts = 0,
}

-- 特殊指令覆蓋表
_G.Practice.special_cmds = {
    ["ventriloquate"] = "cast 'ventriloquate' someone hit me!",
}

-- 已知法術類型 (用於解決 help 指令衝突的問題)
-- 例如 help sleep 會顯示姿勢指令而非法術說明
_G.Practice.known_spell_types = {
    ["sleep"] = "target",    -- cast sleep <victim>
    ["soulsteal"] = "target",
}

-- ===== Hook 系統 =====
-- 確保與 MemCalc 等其他腳本共存
if not _G.Practice.hook_installed then
    local old_hook = _G.on_server_message
    _G.on_server_message = function(line)
        -- 先執行舊的 (例如 MemCalc)
        if old_hook then old_hook(line) end
        -- 再執行我們的
        if _G.Practice and _G.Practice.on_server_message then
            _G.Practice.on_server_message(line)
        end
    end
    _G.Practice.hook_installed = true
end

-- 伺服器訊息處理 (掃描核心)
function _G.Practice.on_server_message(line)
    if not _G.Practice.scan_state.active then return end
    
    local clean_line = string.match(line, "^%s*(.-)%s*$")
    clean_line = string.gsub(clean_line, "\27%[[0-9;]*[mK]", "")
    
    -- 階段 1: 解析 pra 輸出
    if _G.Practice.scan_state.stage == "parsing_pra" then
        -- 格式: [             armor]熟練度:  96/ 882
        for name, prof in string.gmatch(clean_line, "%[%s*(.-)%s*%]熟練度:%s*(%d+)") do
            if name and prof then
                local p = tonumber(prof)
                if p < 99 then
                    table.insert(_G.Practice.scan_state.candidates, name)
                    mud.echo("   收到候選: " .. name .. " (" .. p .. "%)")
                end
            end
        end
        
        -- 偵測結束 Prompt
        -- 用戶範例: (2494/2494 1231/1536 ...)
        -- 舊 Regex: ^%s*%(%d+/%d+hp
        -- 新 Regex: ^%s*%(%d+/%d+
        if string.match(clean_line, "^%s*%(%d+/%d+") then
            if _G.Practice.scan_state.pending_prompts > 0 then
                _G.Practice.scan_state.pending_prompts = _G.Practice.scan_state.pending_prompts - 1
                if _G.Practice.scan_state.pending_prompts <= 0 then
                    mud.echo("📋 列表掃描完成，開始分析 " .. #_G.Practice.scan_state.candidates .. " 個技能...")
                    _G.Practice.start_checking_help()
                end
            end
        end
        return
    end
    
    -- 階段 2: 檢查 help 輸出
    if _G.Practice.scan_state.stage == "checking_help" then
        -- 偵測 "格式" 行
        -- 格式： cast armor <character>
        if string.find(clean_line, "格式") and string.find(clean_line, "cast") then
            local current = _G.Practice.scan_state.current_check
            if not current then return end
            
            -- 判斷類型
            local info = { type = "self" } -- 預設對自己
            
            if string.find(clean_line, "<character>") or string.find(clean_line, "victim") then
                info.type = "target"
            elseif string.find(clean_line, "<object>") or string.find(clean_line, "item") then
                info.type = "object"
            end
            
            -- 加入正式列表 (避免重複)
            local exists = false
            for _, s in ipairs(_G.Practice.spells) do
                if s == current then exists = true break end
            end
            
            if not exists then
                table.insert(_G.Practice.spells, current)
                _G.Practice.spell_info[current] = info
                mud.echo("✅ 加入法術: " .. current .. " (類型: " .. info.type .. ")")
            end
        end
        
        -- 偵測 Prompt (換下一個)
        if string.match(clean_line, "^%s*%(%d+/%d+") then
             if _G.Practice.scan_state.pending_prompts > 0 then
                _G.Practice.scan_state.pending_prompts = _G.Practice.scan_state.pending_prompts - 1
                if _G.Practice.scan_state.pending_prompts <= 0 then
                    _G.Practice.process_next_candidate()
                end
             end
        end
    end
end

-- ===== 掃描邏輯 =====
function _G.Practice.scan()
    mud.echo("🔍 開始自動掃描未滿 99% 的技能...")
    _G.Practice.scan_state = {
        active = true,
        stage = "parsing_pra",
        candidates = {},
        current_check = nil,
        pending_prompts = 1
    }
    _G.Practice.spells = {} -- 清空舊列表 (或者選擇保留?) -> 使用者說 "找出...加入列表"，通常暗示清空重建或追加
    -- 為了乾淨，我們先清空，如果使用者想追加可手動 add
    _G.Practice.spells = {}
    _G.Practice.spell_info = {}
    _G.Practice.index = 1
    
    mud.send("pra")
end

function _G.Practice.start_checking_help()
    _G.Practice.scan_state.stage = "checking_help"
    _G.Practice.process_next_candidate()
end

function _G.Practice.process_next_candidate()
    if #_G.Practice.scan_state.candidates == 0 then
        _G.Practice.finish_scan()
        return
    end
    
    local next_skill = table.remove(_G.Practice.scan_state.candidates, 1)
    
    -- 檢查是否在已知類型表中 (解決 help 衝突問題)
    if _G.Practice.known_spell_types and _G.Practice.known_spell_types[next_skill] then
        local spell_type = _G.Practice.known_spell_types[next_skill]
        
        -- 直接加入，不需要查詢 help
        local exists = false
        for _, s in ipairs(_G.Practice.spells) do
            if s == next_skill then exists = true break end
        end
        
        if not exists then
            table.insert(_G.Practice.spells, next_skill)
            _G.Practice.spell_info[next_skill] = { type = spell_type }
            mud.echo("✅ 加入法術: " .. next_skill .. " (類型: " .. spell_type .. ") [已知]")
        end
        
        -- 繼續下一個
        _G.Practice.process_next_candidate()
        return
    end
    
    _G.Practice.scan_state.current_check = next_skill
    _G.Practice.scan_state.pending_prompts = 1
    
    -- mud.echo("Checking: " .. next_skill)
    mud.send("help " .. next_skill)
end

function _G.Practice.finish_scan()
    _G.Practice.scan_state.active = false
    mud.echo("🎉 掃描完成！共找到 " .. #_G.Practice.spells .. " 個可練習法術。")
    mud.echo("輸入 /lua Practice.start() 開始練習。")
    _G.Practice.status()
end

-- ===== 核心函數 =====

-- 發送指令
local function send_cmd(cmd)
    for part in string.gmatch(cmd, "[^;]+") do
        local clean = string.match(part, "^%s*(.-)%s*$")
        if clean and #clean > 0 then
            mud.send(clean)
        end
    end
end

-- 生成指令 (Updated)
local function build_cmd(spell)
    local P = _G.Practice
    
    -- 1. 特殊指令覆蓋
    if P.special_cmds[spell] then return P.special_cmds[spell] end
    
    -- 2. Soulsteal 特殊處理
    if spell == "soulsteal" then
        local t = {}
        for i = 1, P.soulsteal_count do
            local target = (i == 1) and P.target or (i .. "." .. P.target)
            table.insert(t, "cast 'soulsteal' " .. target)
        end
        return table.concat(t, "; ")
    end

    -- 3. 動態查詢類型
    local info = P.spell_info[spell]
    local type = info and info.type or "self"
    
    -- 相容舊的靜態列表 (如果 spell_info 沒資料)
    -- (這裡為了簡化，如果不 spell_info 裡沒有，就預設 self，或是由使用者 add 進來的)
    -- 為了保險，我們可以保留舊的 hardcode lists，但使用者需求是自動化，所以暫不加入舊代碼的 fallback
    
    if type == "target" then
        return "cast '" .. spell .. "' " .. P.target
    elseif type == "object" then
        return "cast '" .. spell .. "' " .. P.targetobject
    else
        return "cast '" .. spell .. "'"
    end
end

-- 計算延遲
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

    mud.echo(string.format("🔮 [%d/%d] %s (預計耗時 %.1fs)", _G.Practice.index, #_G.Practice.spells, spell, get_spell_delay(spell)))
    send_cmd(build_cmd(spell))

    local delay = get_spell_delay(spell)
    _G.Practice.index = _G.Practice.index + 1
    if _G.Practice.index > #_G.Practice.spells then
        _G.Practice.index = 1
    end
    
    if _G.Practice.running then
        mud.timer(delay, "_G.Practice.loop()")
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
    -- 預設為 self，使用者可能需要手動設，但在這裡先不強求
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

-- 初始化顯示
-- 初始化顯示
local usage = [[
指令:
  1. 自動掃描: /lua Practice.scan()
  2. 開始練習: /lua Practice.start('target')
  3. 停止練習: /lua Practice.stop()
  4. 查看狀態: /lua Practice.status()]]

mud.echo("========================================")
mud.echo("✅ Practice 自動練習腳本 (v2.0 掃描版)")
mud.echo(usage)
mud.echo("========================================")

-- Help 註冊
_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}
_G.Help.registry["Practice"] = {
    desc = "自動練習腳本",
    usage = usage
}
