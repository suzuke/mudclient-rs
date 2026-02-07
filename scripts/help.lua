-- help.lua
-- 統一幫助系統
-- 載入: /lua dofile("help.lua")

_G.Help = _G.Help or {}
_G.Help.registry = _G.Help.registry or {}

-- 列出所有已註冊模組
function _G.Help.list()
    mud.echo("========================================")
    mud.echo("📚 已載入模組清單")
    mud.echo("----------------------------------------")
    
    local count = 0
    local sorted_names = {}
    for name, _ in pairs(_G.Help.registry) do
        table.insert(sorted_names, name)
    end
    table.sort(sorted_names)

    for _, name in ipairs(sorted_names) do
        local info = _G.Help.registry[name]
        mud.echo(string.format("  %-15s : %s", name, info.desc))
        count = count + 1
    end
    
    if count == 0 then
        mud.echo("  (目前沒有模組註冊)")
    end
    mud.echo("----------------------------------------")
    mud.echo("輸入 /lua Help.show('模組名稱') 查看詳細說明")
    mud.echo("========================================")
end

-- 顯示特定模組的詳細說明
function _G.Help.show(name)
    -- 支援不區分大小寫搜尋
    local target = nil
    if _G.Help.registry[name] then
        target = name
    else
        for k, _ in pairs(_G.Help.registry) do
            if string.lower(k) == string.lower(name) then
                target = k
                break
            end
        end
    end

    if target then
        local info = _G.Help.registry[target]
        mud.echo("========================================")
        mud.echo("📘 " .. target .. " - " .. info.desc)
        mud.echo("----------------------------------------")
        mud.echo(info.usage)
        mud.echo("========================================")
    else
        mud.echo("⚠️ 找不到模組: " .. name)
        mud.echo("請使用 /lua Help.list() 查看可用清單")
    end
end

-- 方便的別名
function _G.help()
    _G.Help.list()
end

mud.echo("✅ Help 系統已載入。輸入 /lua help() 查看清單。")
