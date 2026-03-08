-- ============================================================
-- Benumb - 自動施迷香（從箱子中選擇物品）v2.0
-- ============================================================
-- 用法：ben <direction> (建議設定 Alias: ^ben%s+(.+))
-- 使用 mud.collect_response 自動掃描箱子內容
-- 不再需要 hook 和 prompt 偵測
-- ============================================================

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("Benumb cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

-- 移除舊版 hook（如有殘留）
MudUtils.unregister_hook("Benumb")

_G.Benumb = _G.Benumb or {}

-- 可用物品（優先順序）
_G.Benumb.items = {"anesthetic", "grating", "chemical"}

function _G.Benumb.use(dir)
    if not dir or dir == "" then
        mud.echo("用法: ben <direction>")
        return
    end
    mud.echo("🔍 Benumb: 掃描箱子 → 方向 [" .. dir .. "]...")
    local safe_dir = dir:gsub("'", "")
    mud.collect_response("l in box", function(lines)
        _G.Benumb._on_box_scanned(dir, lines)
    end)
end

function _G.Benumb._on_box_scanned(dir, lines)
    lines = lines or {}
    local chosen = nil

    for _, item in ipairs(_G.Benumb.items) do
        for _, line in ipairs(lines) do
            if string.find(line, item) then
                chosen = item
                break
            end
        end
        if chosen then break end
    end

    if chosen then
        mud.echo("🧪 使用 " .. chosen .. " → " .. dir)
        mud.send("get " .. chosen .. " box")
        mud.send("benumb " .. chosen .. " " .. dir)
    else
        mud.echo("❌ 箱子裡沒有可用的迷香物品")
    end
end

function _G.Benumb.reload()
    package.loaded["scripts.benumb"] = nil
    require("scripts.benumb")
    mud.echo("[Benumb] ♻️ 腳本已重新載入")
end

mud.echo("[Benumb] 已載入 (v2.0)。用法: ben <direction>")
