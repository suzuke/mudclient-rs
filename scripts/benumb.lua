-- ============================================================
-- Benumb - 自動施迷香（從箱子中選擇物品）
-- ============================================================
-- 用法：ben <direction>
-- 自動偵測 box 中的物品，按優先順序選擇使用
-- ============================================================

_G.Benumb = _G.Benumb or {}

-- 可用物品（優先順序）
_G.Benumb.items = {"anesthetic", "grating", "chemical"}

-- 狀態
_G.Benumb.pending_dir = nil
_G.Benumb.found_items = {}
_G.Benumb.scanning = false

function _G.Benumb.use(dir)
    if not dir or dir == "" then
        mud.echo("用法: ben <direction>")
        return
    end
    _G.Benumb.pending_dir = dir
    _G.Benumb.found_items = {}
    _G.Benumb.scanning = true
    mud.send("l in box")
end

-- Hook
if not _G.Benumb.hook_installed then
    local old_hook = _G.on_server_message
    _G.on_server_message = function(line)
        if old_hook then old_hook(line) end
        if _G.Benumb and _G.Benumb.on_msg then
            _G.Benumb.on_msg(line)
        end
    end
    _G.Benumb.hook_installed = true
end

function _G.Benumb.on_msg(line)
    if not _G.Benumb.scanning then return end

    local clean = line:gsub("\27%[[0-9;]*m", "")

    -- 掃描箱子內容，記錄找到的物品
    for _, item in ipairs(_G.Benumb.items) do
        if string.find(clean, item) then
            _G.Benumb.found_items[item] = true
        end
    end

    -- 偵測 prompt → 箱子內容列表結束
    if (string.find(clean, "hp%d+/%d+") or string.find(clean, "%d+/%d+hp")) and _G.Benumb.pending_dir then
        _G.Benumb.scanning = false

        -- 按優先順序選擇第一個可用物品
        local chosen = nil
        for _, item in ipairs(_G.Benumb.items) do
            if _G.Benumb.found_items[item] then
                chosen = item
                break
            end
        end

        if chosen then
            local dir = _G.Benumb.pending_dir
            mud.echo("🧪 使用 " .. chosen .. " → " .. dir)
            mud.send("get " .. chosen .. " box")
            mud.send("benumb " .. chosen .. " " .. dir)
        else
            mud.echo("❌ 箱子裡沒有可用的迷香物品")
        end

        _G.Benumb.pending_dir = nil
        _G.Benumb.found_items = {}
    end
end

mud.echo("[Benumb] 已載入。用法: ben <direction>")
