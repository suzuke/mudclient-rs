-- ============================================================
-- Benumb - 自動施迷香（從箱子中選擇物品）
-- ============================================================
-- 用法：ben <direction> (建議設定 Alias: ^ben%s+(.+))
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
    mud.echo("🔍 Benumb: 準備在方向 [" .. dir .. "] 使用迷香...")
    _G.Benumb.pending_dir = dir
    _G.Benumb.found_items = {}
    _G.Benumb.scanning = true
    mud.send("l in box")
end

-- ===== Hook Registry =====
MudUtils.register_hook("Benumb", function(line, clean_line)
    _G.Benumb.on_msg(line, clean_line)
end)

function _G.Benumb.on_msg(line, clean_line)
    if not _G.Benumb.scanning then return end

    local clean = clean_line -- 直接使用 Rust 傳入的 clean_line
    -- local clean = line:gsub("\27%[[0-9;]*m", "")

    -- 掃描箱子內容，記錄找到的物品
    for _, item in ipairs(_G.Benumb.items) do
        if string.find(clean, item) then
            _G.Benumb.found_items[item] = true
        end
    end

    -- 偵測 prompt → 箱子內容列表結束
    -- 放寬判定：支援 > 開頭, [ 開頭, 或包含 hp/HP 的行
    local is_prompt = string.find(clean, "^>") or 
                      string.find(clean, "^%[") or 
                      string.find(clean, "^%*") or -- 某些 MUD 的忙碌/戰鬥提示
                      string.find(clean, "hp%d+") or 
                      string.find(clean, "%d+/%d+") or
                      string.lower(clean):find("hp:")

    if is_prompt and _G.Benumb.pending_dir then
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

function _G.Benumb.reload()
    package.loaded["scripts.benumb"] = nil
    require("scripts.benumb")
    mud.echo("[Benumb] ♻️ 腳本已重新載入")
end

mud.echo("[Benumb] 已載入。用法: ben <direction>")
