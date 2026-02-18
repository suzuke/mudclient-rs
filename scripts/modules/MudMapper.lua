-- MudMapper: 自動繪圖與路徑記錄器
-- 使用方式: /lua mapper.start()

local MudMapper = {}
_G.MudMapper = MudMapper

-- 設定檔路徑
MudMapper.SAVE_FILE = "data/mapper_data.json"

MudMapper.state = {
    running = false,
    current_id = nil,
    last_id = nil,
    last_move_dir = nil,
    last_move_time = 0,
    
    rooms = {},   -- [id] = { name, desc, exits_list, visited_count }
    moves = {},   -- [from_id] = { [dir] = to_id }
    history = {}, -- list of {id, name, time}
}

-- 方向對應表 (用於反向連接)
MudMapper.REVERSE_DIR = {
    ["n"] = "s", ["s"] = "n", ["e"] = "w", ["w"] = "e",
    ["u"] = "d", ["d"] = "u",
    ["ne"] = "sw", ["sw"] = "ne", ["nw"] = "se", ["se"] = "nw",
    ["north"] = "south", ["south"] = "north", ["east"] = "west", ["west"] = "east",
    ["up"] = "down", ["down"] = "up"
}

function MudMapper.goto_room(target)
    if not MudMapper.state.current_id then
         mud.echo("{r[Map] 錯誤: 尚未定位。請先移動一步或輸入 'l' 以更新位置。{x}")
         return
    end

    local path_str = MudMapper.show_path(target)
    if path_str then
        mud.echo("{g[Map] 開始導航...{x}")
        for cmd in string.gmatch(path_str, "[^;]+") do
            mud.send(cmd)
        end
    else
        mud.echo(string.format("{r[Map] 導航失敗。請確認目的地 '%s' 是否已在已探索地圖中。{x}", target))
        mud.echo("{w(提示: 您可以使用 /lua mapper.status() 查看已探索的房間數){x}")
    end
end

-- 標準化方向
function MudMapper.normalize_dir(dir)
    dir = string.lower(dir)
    local map = {
        ["north"] = "n", ["south"] = "s", ["east"] = "e", ["west"] = "w",
        ["up"] = "u", ["down"] = "d",
        ["northeast"] = "ne", ["northwest"] = "nw", ["southeast"] = "se", ["southwest"] = "sw"
    }
    return map[dir] or dir
end

function MudMapper.start()
    if MudMapper.state.running then
        mud.echo("{yMudMapper 已經在執行中。{x}")
        return
    end
    
    MudMapper.load() -- 嘗試載入存檔
    MudMapper.state.running = true
    mud.echo("{gMudMapper 已啟動。開始自動繪圖...{x}")
    
    -- 註冊 Hook
    if not MudMapper.hooks_registered then
        MudMapper.register_hooks()
        MudMapper.hooks_registered = true
    end
end

function MudMapper.stop()
    MudMapper.state.running = false
    MudMapper.save() -- 自動存檔
    mud.echo("{rMudMapper 已停止 (已自動存檔)。{x}")
end

function MudMapper.register_hooks()
    -- 1. on_room_detected
    if not MudMapper.original_on_room_detected then
        MudMapper.original_on_room_detected = _G.on_room_detected
    end
    
    _G.on_room_detected = function(id, name)
        if MudMapper.original_on_room_detected then
            pcall(MudMapper.original_on_room_detected, id, name)
        end
        
        if MudMapper.state.running then
            MudMapper.on_room(id, name)
        end
    end
    
    -- 2. on_command (需要在 Rust 端支援，如果沒有支援則無效)
    if not MudMapper.original_on_command then
        MudMapper.original_on_command = _G.on_command
    end
    
    _G.on_command = function(cmd)
        if MudMapper.original_on_command then
            pcall(MudMapper.original_on_command, cmd)
        end
        
        if MudMapper.state.running then
            MudMapper.on_command(cmd)
        end
    end
end

function MudMapper.on_command(cmd)
    local cmd = string.lower(tostring(cmd))
    
    -- 檢查是否為移動指令
    local dir = MudMapper.normalize_dir(cmd)
    if MudMapper.REVERSE_DIR[MudMapper.normalize_dir(cmd)] or MudMapper.REVERSE_DIR[cmd] then
        -- 記錄預期移動
        MudMapper.state.last_move_dir = dir
        MudMapper.state.last_move_time = os.time()
        -- mud.echo("Debug: Move " .. dir)
    else
        -- 非移動指令，重置 (或是保留？有時候移動後會 lag)
        -- 為了避免誤判，如果指令不是看/查狀態等，可能就要重置
        -- 暫時不重置，依賴時間判斷
    end
end

function MudMapper.on_room(id, name)
    local s = MudMapper.state
    local now = os.time()
    
    -- 更新當前房間資訊
    s.current_id = id
    
    -- 記錄房間基本資料
    if not s.rooms[id] then
        s.rooms[id] = {
            id = id,
            name = name,
            first_visit = now,
            visited = 0,
            exits = {} -- 這裡可以透過 mud.get_current_room().exits 更新
        }
        mud.echo(string.format("{G[✨ 新地圖節點] %s (ID: %s...){x}", name, string.sub(id, 1, 6)))
    end
    
    s.rooms[id].visited = (s.rooms[id].visited or 0) + 1
    s.rooms[id].name = name -- 更新名稱，以防變更
    
    -- 嘗試建立連接
    if s.last_id and s.last_move_dir then
        -- 檢查時間差，避免過久的移動指令被誤用 (例如 5 秒前輸入 n，現在才有人傳送過來)
        if now - s.last_move_time <= 5 then
            local from = s.last_id
            local to = id
            local dir = s.last_move_dir
            
            -- 記錄正向連接
            if not s.moves[from] then s.moves[from] = {} end
            if s.moves[from][dir] ~= to then
                s.moves[from][dir] = to
                mud.echo(string.format("{c[Map] 連接: %s --%s--> %s{x}", s.rooms[from].name, dir, name))
            end
            
            -- 記錄反向連接 (選擇性，MUD 不一定是雙向的，但通常是)
            -- 我們標記為 "推測" 或者直接記錄
            -- 這裡先不自動記錄反向，以免單向門導致地圖錯誤
        end
    end
    
    -- 更新狀態
    s.last_id = id
    s.last_move_dir = nil -- 清除移動意圖
    
    -- 麵包屑提示
    if s.rooms[id].visited > 1 and s.rooms[id].visited % 10 == 0 then
        mud.echo(string.format("{y[Map] 這裡是第 %d 次訪問 %s{x}", s.rooms[id].visited, name))
    end
end

-- ===== 存檔/讀檔 (簡單 JSON) =====
function MudMapper.save()
    if not mud or not mud.start_log then return end -- 檢查環境
    
    -- 由於 Lua 沒有內建 JSON，我們手刻一個簡單的序列化 (僅支援 table, string, number)
    -- 或者使用簡單的 Lua table dump
    
    local function serialize(o)
        if type(o) == "number" then
            return tostring(o)
        elseif type(o) == "string" then
            return string.format("%q", o)
        elseif type(o) == "table" then
            local s = "{"
            for k,v in pairs(o) do
                if type(k) == "string" then
                    s = s .. "[" .. string.format("%q", k) .. "]=" .. serialize(v) .. ","
                elseif type(k) == "number" then
                    s = s .. "[" .. k .. "]=" .. serialize(v) .. ","
                end
            end
            return s .. "}"
        else
            return "nil"
        end
    end
    
    -- 我們將資料存為 Lua code，讀取時直接 loadstring
    -- 為了避免檔案過大，只存 rooms 和 moves
    local data = {
        rooms = MudMapper.state.rooms,
        moves = MudMapper.state.moves
    }
    
    local content = "return " .. serialize(data)
    
    local f = io.open(MudMapper.SAVE_FILE, "w")
    if f then
        f:write(content)
        f:close()
        mud.echo("{w[Map] 地圖資料已儲存至 " .. MudMapper.SAVE_FILE .. "{x}")
    else
        mud.echo("{R[Map] 存檔失敗！無法寫入檔案。{x}")
    end
end

function MudMapper.load()
    local f = io.open(MudMapper.SAVE_FILE, "r")
    if f then
        local content = f:read("*a")
        f:close()
        
        local func, err = load(content)
        if func then
            local status, data = pcall(func)
            if status and data then
                MudMapper.state.rooms = data.rooms or {}
                MudMapper.state.moves = data.moves or {}
                mud.echo(string.format("{w[Map] 已載入地圖資料 (房間數: %d){x}", 
                    (function() local c=0; for _ in pairs(data.rooms) do c=c+1 end return c end)()))
            else
                mud.echo("{R[Map] 讀檔失敗: 格式錯誤{x}")
            end
        else
            mud.echo("{R[Map] 讀檔失敗: " .. tostring(err) .. "{x}")
        end
    end
end

function MudMapper.reload()
    if MudMapper.state.running then MudMapper.stop() end
    package.loaded["modules.MudMapper"] = nil
    local status, new_module = pcall(require, "modules.MudMapper")
    if status then
        mud.echo("{gMudMapper 重新載入成功。{x}")
        -- new_module.start() -- 自動重啟?
    else
        mud.echo("{rMudMapper 重新載入失敗: " .. tostring(new_module) .. "{x}")
    end
end

function MudMapper.show_exits(id)
    id = id or MudMapper.state.current_id
    if not id or not MudMapper.state.moves[id] then
        mud.echo("沒有此房間的出口記錄。")
        return
    end
    
    mud.echo(string.format("{c房間 %s 的已知連接:{x}", MudMapper.state.rooms[id].name))
    for dir, to_id in pairs(MudMapper.state.moves[id]) do
        local target = MudMapper.state.rooms[to_id]
        local name = target and target.name or "未知房間"
        mud.echo(string.format("  {G%s{x} -> %s", dir, name))
    end
end

-- BFS 搜尋路徑
function MudMapper.find_path(target_id)
    local start_id = MudMapper.state.current_id
    if not start_id then
        mud.echo("{r錯誤: 未知當前位置，請先移動以定位。{x}")
        return nil
    end
    
    if start_id == target_id then
        return {}
    end
    
    local queue = {{id = start_id, path = {}}}
    local visited = {[start_id] = true}
    
    local head = 1
    while head <= #queue do
        local current = queue[head]
        head = head + 1
        
        local current_id = current.id
        local neighbors = MudMapper.state.moves[current_id]
        
        if neighbors then
            for dir, next_id in pairs(neighbors) do
                if not visited[next_id] then
                    visited[next_id] = true
                    
                    -- 複製路徑
                    local new_path = {}
                    for _, p in ipairs(current.path) do table.insert(new_path, p) end
                    table.insert(new_path, dir)
                    
                    if next_id == target_id then
                        return new_path
                    end
                    
                    table.insert(queue, {id = next_id, path = new_path})
                end
            end
        end

    end
    
    return nil
end

-- 搜尋目標房間 (處理重複名稱)
function MudMapper.resolve_target(input)
    -- 1. 精確 ID 匹配 (最優先)
    if MudMapper.state.rooms[input] then
        return input, MudMapper.state.rooms[input].name
    end

    -- 2. 搜尋所有可能的匹配 (部分 ID 或 名稱)
    local matches = {}
    for id, room in pairs(MudMapper.state.rooms) do
        -- 支援部分 ID 匹配
        if string.find(id, input, 1, true) then
            table.insert(matches, {id = id, name = room.name, accuracy = "partial_id"})
        -- 或是名稱匹配
        elseif string.find(room.name, input, 1, true) then
            table.insert(matches, {id = id, name = room.name, accuracy = "name"})
        end
    end

    if #matches == 0 then
        return nil, "找不到 ID 或名稱包含 '" .. input .. "' 的房間。"
    elseif #matches == 1 then
        return matches[1].id, matches[1].name
    else
        -- 歧義：顯示所有選項
        mud.echo(string.format("{y[Map] 找到 %d 個符合 '%s' 的房間，請使用 ID 指定:{x}", #matches, input))
        for _, m in ipairs(matches) do
            mud.echo(string.format("  {c%s{x} - %s", m.id, m.name))
        end
        return nil, "目標不明確，請輸入完整 ID。"
    end
end

function MudMapper.find_target_command(input)
    if not input then 
        mud.echo("請輸入要搜尋的關鍵字 (ID 或 名稱)。")
        return
    end
    local id, name_or_err = MudMapper.resolve_target(input)
    if id then
        mud.echo(string.format("{g[Map] 鎖定目標: %s (ID: %s){x}", name_or_err, id))
    else
        mud.echo("{r[Map] " .. name_or_err .. "{x}")
    end
end

function MudMapper.show_path(target)
    if not target then
        mud.echo("請輸入目的地 (ID 或 名稱)。")
        return nil
    end

    local target_id, target_name_or_err = MudMapper.resolve_target(target)
    
    if not target_id then
        -- resolve_target 已經 print 了歧義列表或錯誤
        if target_name_or_err then
             mud.echo("{r[Map] " .. target_name_or_err .. "{x}")
        end
        return nil
    end
    
    local target_name = target_name_or_err
    local path = MudMapper.find_path(target_id)
    
    if path then
        mud.echo(string.format("{c路徑至 %s (%s):{x}", target_name, target_id))
        local path_str = table.concat(path, ";")
        mud.echo(string.format("{G%s{x}", path_str))
        return path_str
    else
        mud.echo(string.format("{r無法找到通往 %s 的路徑 (可能是孤島或未探索連接)。{x}", target_name))
        return nil
    end
end

function MudMapper.goto_room(target)
    if not MudMapper.state.current_id then
         mud.echo("{r[Map] 錯誤: 尚未定位。請先移動一步或輸入 'l' 以更新位置。{x}")
         return
    end

    local path_str = MudMapper.show_path(target)
    if path_str then
        mud.echo("{g[Map] 開始導航...{x}")
        for cmd in string.gmatch(path_str, "[^;]+") do
            mud.send(cmd)
        end
    end
end

_G.mapper = {
    start = MudMapper.start,
    stop = MudMapper.stop,
    save = MudMapper.save,
    load = MudMapper.load,
    status = function() 
        local rc = 0; for _ in pairs(MudMapper.state.rooms) do rc=rc+1 end
        mud.echo(string.format("已記錄房間: %d", rc))
    end,
    reload = MudMapper.reload,
    exits = MudMapper.show_exits,
    path = MudMapper.show_path,
    go = MudMapper.goto_room,
    find = MudMapper.find_target_command
}

return MudMapper
