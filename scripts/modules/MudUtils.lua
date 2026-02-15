-- MudUtils Module
local MudUtils = {}
_G.MudUtils = MudUtils -- Export to global for timer callbacks

MudUtils.run_id = 0
MudUtils.callbacks = {}
MudUtils.callback_id = 0
MudUtils.active_quests = {} -- { [name] = stop_fn }
MudUtils.inventory_parsing = false
MudUtils.has_life_crystal = true -- 預設為 true，直到檢查失敗

function MudUtils.get_new_run_id()
    MudUtils.run_id = MudUtils.run_id + 1
    return MudUtils.run_id
end

function MudUtils.check_run(rid)
    return rid == MudUtils.run_id
end

function MudUtils.parse_cmds(str)
    local result = {}
    for cmd in string.gmatch(str, "[^;]+") do
        cmd = cmd:match("^%s*(.-)%s*$")
        if cmd ~= "" then
            local count, actual = cmd:match("^(%d+)(%a.*)$")
            if count then
                for _ = 1, tonumber(count) do
                    table.insert(result, actual)
                end
            else
                table.insert(result, cmd)
            end
        end
    end
    return result
end

function MudUtils.safe_timer(seconds, callback)
    local rid = MudUtils.run_id
    MudUtils.callback_id = MudUtils.callback_id + 1
    local set_cb_id = MudUtils.callback_id
    
    MudUtils.callbacks[set_cb_id] = callback
    
    -- In real MUD this is a string to execute
    -- In our mock, we can capture this string
    local code = "_G.MudUtils.exec_timer(" .. set_cb_id .. ", " .. rid .. ")"
    if mud and mud.timer then
        mud.timer(seconds, code)
    end
end

function MudUtils.exec_timer(cb_id, rid)
    if not MudUtils.check_run(rid) then 
        MudUtils.callbacks[cb_id] = nil
        return 
    end
    
    local cb = MudUtils.callbacks[cb_id]
    if cb then
        cb(rid)
        MudUtils.callbacks[cb_id] = nil
    end
end

-- Standardized Script Help Output
function MudUtils.print_script_help(name, version, description, commands)
    if not mud then return end
    
    mud.echo("════════════════════════════════════════════════════")
    mud.echo("  📜 " .. name .. " " .. (version or ""))
    mud.echo("════════════════════════════════════════════════════")
    if description then
        mud.echo("  " .. description)
        mud.echo("────────────────────────────────────────────────")
    end
    
    if commands then
        mud.echo("  📌 可用指令:")
        for _, cmd_def in ipairs(commands) do
            local cmd = cmd_def.cmd or cmd_def[1] or ""
            local desc = cmd_def.desc or cmd_def[2] or ""
            -- Simple padding
            local padding = ""
            if #cmd < 25 then
                padding = string.rep(" ", 25 - #cmd)
            end
            mud.echo("    " .. cmd .. padding .. " - " .. desc)
        end
    end
    mud.echo("════════════════════════════════════════════════════")
end

function MudUtils.show_script_usage(name, usage_lines)
    if not mud then return end
    mud.echo("📜 " .. name)
    mud.echo("════════════════════════════════════════════════════")
    for _, line in ipairs(usage_lines) do
        mud.echo("  " .. line)
    end
    mud.echo("════════════════════════════════════════════════════")
end

function MudUtils.send_cmds(str)
    local cmds = MudUtils.parse_cmds(str)
    for _, cmd in ipairs(cmds) do
        mud.send(cmd)
    end
end

-- 腳本日誌管理
function MudUtils.start_log(prefix)
    if mud and mud.start_log then
        local ts = os.time()
        local filename = string.format("%s_%s.txt", prefix, ts)
        mud.start_log(filename)
        mud.echo("📝 [Log] 開始紀錄至 " .. filename)
    end
end

function MudUtils.stop_log()
    if mud and mud.stop_log then
        mud.stop_log()
        mud.echo("📝 [Log] 停止紀錄")
    end
end

-- ===== 全局任務管理 =====

function MudUtils.register_quest(name, stop_fn)
    MudUtils.active_quests[name] = stop_fn
end

function MudUtils.halt_all_quests(reason)
    local any_stopped = false
    for name, stop_fn in pairs(MudUtils.active_quests) do
        if stop_fn then
            pcall(stop_fn)
            any_stopped = true
        end
    end
    
    if any_stopped then
        mud.echo("════════════════════════════════════════")
        mud.echo("🚨 [全局停止] 原因: " .. (reason or "未知"))
        mud.echo("════════════════════════════════════════")
    end
end

-- ===== 物品檢查邏輯 =====

function MudUtils.on_server_message(line, clean_line)
    -- 偵測物品清單開始
    if string.find(clean_line, "你身上攜帶著有:", 1, true) then
        MudUtils.inventory_parsing = true
        MudUtils.temp_has_crystal = false
        return
    end

    if MudUtils.inventory_parsing then
        -- 檢查是否包含生命水晶
        if string.find(clean_line, "生命水晶", 1, true) or string.find(clean_line, "life crystal", 1, true) then
            MudUtils.temp_has_crystal = true
        end

        -- 偵測清單結束 (通常是空行或特定結尾，這裡簡單處理：只要有內容就繼續，若遇到 Ok. 或指令提示點則結束)
        -- 但 MUD 的 i 通常很短。我們可以設定一個短延遲後的評估，或是偵測下一行。
        -- 這裡改用通用 Hook 觸發：如果下一行是空行或包含特定特徵，則評估。
        -- 更簡單做法：每看到一行就檢查，並用一個 timer (0.1s) 延遲評估，新的行會重設 timer。
        
        if MudUtils.inv_timer_id then mud.timer_stop(MudUtils.inv_timer_id) end
        MudUtils.inv_timer_id = MudUtils.safe_timer(0.2, function()
            MudUtils.inventory_parsing = false
            MudUtils.has_life_crystal = MudUtils.temp_has_crystal
            if not MudUtils.has_life_crystal then
                MudUtils.halt_all_quests("身上未攜帶生命水晶！")
            end
        end)
    end
end

-- 為了方便整合，MudUtils 也可以被註冊到全域 Hook
-- 修正：避免巢狀包裹 (Nesting)
if not MudUtils.hook_installed then
    if _G.on_server_message ~= MudUtils.on_server_message then
        MudUtils._base_hook = _G.on_server_message
        _G.on_server_message = function(line, clean_line)
            if MudUtils._base_hook then 
                local status, err = pcall(MudUtils._base_hook, line, clean_line)
                if not status then mud.echo("MudUtils Base Hook Error: " .. tostring(err)) end
            end
            local status, err = pcall(MudUtils.on_server_message, line, clean_line)
            if not status then mud.echo("MudUtils Message Hook Error: " .. tostring(err)) end
        end
        MudUtils.hook_installed = true
    end
end

return MudUtils
