-- ============================================================
-- MobFinder - 指定 Mob 搜尋腳本 (Refactored)
-- ============================================================
-- 使用 MudExplorer (DFS) 與 MudNav (導航)
-- ============================================================

_G.MobFinder = _G.MobFinder or {}

-- Robust require function
local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local status, res = pcall(require, p)
        if status then return res end
    end
    error("MobFinder cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")
local MudNav = require_module("MudNav")
local MudExplorer = require_module("MudExplorer")

-- Force reload if needed (for dev)
if not MudUtils.print_script_help then
    package.loaded["scripts/modules/MudUtils"] = nil
    MudUtils = require_module("MudUtils")
end

-- ===== 設定 =====
_G.MobFinder.config = {
    target = "otonashi",          
    entry_path = "6w;3n;enter ikkoku",  
    enter_cmds = {},               
    on_found = nil,                
    max_laps = 5,                  
    debug = false,
}

-- ===== 狀態 =====
_G.MobFinder.state = {
    running = false,
    -- run_id handled by MudUtils
    phase = "idle",      -- idle / entering / explore / found
}

-- ===== 訊息輸出 =====
function _G.MobFinder.echo(msg)
    mud.echo("[MobFinder] " .. msg)
end

-- ===== 核心邏輯 =====

function _G.MobFinder.start(target)
    if _G.MobFinder.state.running then
        _G.MobFinder.echo("⚠️ 搜尋已在執行中")
        return
    end

    _G.MobFinder.reset_state()
    MudUtils.get_new_run_id()
    local rid = MudUtils.run_id
    
    local s = _G.MobFinder.state
    s.running = true
    
    if target then _G.MobFinder.config.target = target end
    
    _G.MobFinder.echo("═══════════════════════════════════════")
    _G.MobFinder.echo("🔍 MobFinder 啟動！ v0.2")
    _G.MobFinder.echo("   目標: " .. _G.MobFinder.config.target)
    _G.MobFinder.echo("   路徑: " .. _G.MobFinder.config.entry_path)
    _G.MobFinder.echo("═══════════════════════════════════════")
    MudUtils.start_log("mobfinder")

    mud.send("repo")
    mud.send("wa")
    mud.send("recall")
    
    MudUtils.safe_timer(1.5, function(new_rid)
        _G.MobFinder.enter_area(new_rid)
    end)
end

function _G.MobFinder.enter_area(rid)
    if not MudUtils.check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end
    
    s.phase = "entering"
    _G.MobFinder.echo("🚀 前往目標區域...")
    
    MudNav.walk(_G.MobFinder.config.entry_path, function()
        _G.MobFinder.start_explore(rid)
    end)
end

function _G.MobFinder.start_explore(rid)
    if not MudUtils.check_run(rid) then return end
    local s = _G.MobFinder.state
    if not s.running then return end
    
    -- Execute extra enter commands
    local cmds = _G.MobFinder.config.enter_cmds or {}
    for _, cmd in ipairs(cmds) do
        mud.send(cmd)
    end
    
    s.phase = "explore"
    _G.MobFinder.echo("🕵️ 開始 DFS 探索...")
    
    -- Configure Explorer
    MudExplorer.config.target = _G.MobFinder.config.target
    MudExplorer.config.max_laps = _G.MobFinder.config.max_laps
    
    -- Start Explorer
    MudExplorer.explore(function(found, target_line)
        if found then
            s.phase = "found"
            _G.MobFinder.echo("🎉 搜尋成功！目標在場。")
            _G.MobFinder.echo("   " .. (target_line or ""))
             -- execute callback if any
            if _G.MobFinder.config.on_found then
                _G.MobFinder.config.on_found()
            end
            _G.MobFinder.stop()
        else
            _G.MobFinder.echo("❌ 搜尋失敗，未找到目標。")
            _G.MobFinder.stop()
        end
    end)
end

function _G.MobFinder.stop()
    local s = _G.MobFinder.state
    s.running = false
    s.phase = "idle"
    MudNav.state.walking = false
    MudExplorer.state.exploring = false
    _G.MobFinder.echo("🛑 搜尋已停止")
    MudUtils.stop_log()
end

function _G.MobFinder.reset_state()
    _G.MobFinder.state.running = false
    _G.MobFinder.state.phase = "idle"
end

function _G.MobFinder.status()
    local s = _G.MobFinder.state
    _G.MobFinder.echo("📊 狀態:")
    _G.MobFinder.echo("   執行中: " .. (s.running and "是" or "否"))
    _G.MobFinder.echo("   階段: " .. s.phase)
    _G.MobFinder.echo("   目標: " .. _G.MobFinder.config.target)
    
    if s.phase == "explore" then
        mud.echo("   --- Explorer Status ---")
        MudExplorer.status()
    end
end

function _G.MobFinder.reload()
    package.loaded["scripts.mob_finder"] = nil
    require("scripts.mob_finder")
    _G.MobFinder.echo("♻️ 腳本已重新載入")
end

-- ===== Hook =====
-- 為了避免重複包裝 (Nesting)，我們需要更謹慎地處理 Hook
if _G.MobFinder.hook_installed and _G.MobFinder._original_hook then
    _G.on_server_message = _G.MobFinder._original_hook
end
if not _G.MobFinder._original_hook then
    _G.MobFinder._original_hook = _G.on_server_message
end
local base_hook = _G.MobFinder._original_hook

_G.on_server_message = function(line, clean_line)
    local status, err = pcall(function()
        if base_hook then base_hook(line, clean_line) end
        -- Global Hook Delegation
        MudNav.on_server_message(clean_line)
        MudExplorer.on_server_message(clean_line)
    end)
    if not status then
        mud.echo("CRITICAL HOOK ERROR (MobFinder): " .. tostring(err))
    end
end
_G.MobFinder.hook_installed = true

-- ===== 自動執行 =====
MudUtils.print_script_help(
    "MobFinder", 
    "v0.2 (Refactored)", 
    "指定 Mob 搜尋腳本 (整合 MudExplorer)",
    {
        {cmd="MobFinder.start(target)", desc="🚀 開始搜尋 (可選 target)"},
        {cmd="MobFinder.stop()",  desc="🛑 停止搜尋"},
        {cmd="MobFinder.status()", desc="📊 查看狀態"},
        {cmd="MobFinder.reload()", desc="♻️ 重新載入腳本"},
    }
)

return _G.MobFinder

