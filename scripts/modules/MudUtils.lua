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

function MudUtils.string_to_hex(str)
    return (str:gsub('.', function (c)
        return string.format('%02X ', string.byte(c))
    end))
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
        return mud.timer(seconds, code)
    end
    return nil
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
        -- Ensure logs directory exists or use it
        local filename = string.format("logs/%s_%s.txt", prefix, ts)
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
            any_stopped = true
            break
        end
    end
    
    if any_stopped then
        mud.echo("════════════════════════════════════════")
        mud.echo("🚨 [全局停止] 原因: " .. (reason or "未知"))
        mud.echo("════════════════════════════════════════")
    end

    for name, stop_fn in pairs(MudUtils.active_quests) do
        if stop_fn then
            pcall(stop_fn)
        end
    end
end

-- ===== 物品檢查邏輯 =====
-- 使用 Rust 端 mud.collect_response 收集完整的 inventory 回應
-- Rust 偵測 prompt（不完整行）作為回應結束標記，萬無一失

function MudUtils.start_inventory_check(on_complete_callback)
    MudUtils._inventory_callback = on_complete_callback
    mud.collect_response("i", "_G.MudUtils._on_inventory_collected()")
end

function MudUtils._on_inventory_collected()
    local lines = _G._collected_lines or {}
    local found = false
    for _, line in ipairs(lines) do
        if string.find(line, "生命水晶", 1, true) or
           string.find(line, "life crystal", 1, true) then
            found = true
            break
        end
    end
    MudUtils.has_life_crystal = found
    if not found then
        MudUtils.halt_all_quests("身上未攜帶生命水晶！")
    else
        if MudUtils._inventory_callback then
            if type(MudUtils._inventory_callback) == "string" then
                -- 使用 0 秒計時器確保在目前執行緒脈絡外執行，避免錯誤中斷當前流程
                mud.timer(0.1, MudUtils._inventory_callback)
            elseif type(MudUtils._inventory_callback) == "function" then
                pcall(MudUtils._inventory_callback)
            end
        end
    end
end

function MudUtils.on_server_message(line, clean_line)
    -- 目前無需在 hook 中做 inventory 解析
    -- 保留此函數供未來擴充
end

-- ===== Hook Registry =====
-- 統一的 on_server_message hook 管理，避免 reload 後巢狀疊加

MudUtils.hook_registry = MudUtils.hook_registry or {}

--- 註冊一個 on_server_message hook
--- @param name string 唯一名稱（重複註冊會覆蓋舊的）
--- @param fn function(line, clean_line) 回呼函數
function MudUtils.register_hook(name, fn)
    MudUtils.hook_registry[name] = fn
end

--- 移除指定的 hook
--- @param name string
function MudUtils.unregister_hook(name)
    MudUtils.hook_registry[name] = nil
end

-- 自動註冊 MudUtils 自己的 hook
MudUtils.register_hook("MudUtils", MudUtils.on_server_message)

-- 設定全域 on_server_message 為 registry dispatcher
-- 只設定一次，reload 時 registry 內容會被覆蓋但 dispatcher 不變
if not MudUtils._dispatcher_installed then
    _G.on_server_message = function(line, clean_line)
        for name, fn in pairs(MudUtils.hook_registry) do
            local ok, err = pcall(fn, line, clean_line)
            if not ok and mud then
                mud.echo("[Hook Error: " .. name .. "] " .. tostring(err))
            end
        end
    end
    MudUtils._dispatcher_installed = true
end

return MudUtils

