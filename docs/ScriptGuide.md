# MudClient Lua 腳本撰寫指南

> **適用版本**：mudclient-rs（Rust + Lua 架構）  
> **最後更新**：2026-02-20

---

## 目錄

1. [目錄結構與載入順序](#目錄結構與載入順序)
2. [腳本範本](#腳本範本)
3. [顯式依賴宣告（必要）](#顯式依賴宣告必要)
4. [模組介紹](#模組介紹)
5. [Hook 機制](#hook-機制)
6. [全域命名空間規範](#全域命名空間規範)
7. [Timer 與 run_id 防競態](#timer-與-run_id-防競態)
8. [reload() 標準實作](#reload-標準實作)
9. [腳本載入流程一覽](#腳本載入流程一覽)

---

## 目錄結構與載入順序

```
scripts/
├── modules/        # ★ Phase 1：優先載入（依字母排序）
│   ├── MudCombat.lua
│   ├── MudExplorer.lua
│   ├── MudLoot.lua
│   ├── MudMapper.lua
│   ├── MudNav.lua
│   └── MudUtils.lua
├── *.lua           # Phase 2：頂層腳本載入（依字母排序）
└── tests/          # 測試腳本（不自動載入）
```

> **重要**：Rust 層保證 `modules/` 在所有頂層腳本之前載入。  
> 即便如此，**每個腳本仍應顯式宣告依賴**，原因見下節。

---

## 腳本範本

以下是新腳本的標準起手式，請直接複製修改：

```lua
-- ============================================================
-- MyScript - 說明
-- ============================================================
-- 使用: /lua MyScript.start()
-- 停止: /lua MyScript.stop()
-- ============================================================

-- ===== 依賴宣告（必要）=====
local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("MyScript cannot load dependency: " .. name)
end

local MudUtils  = require_module("MudUtils")
-- local MudNav    = require_module("MudNav")      -- 按需取消
-- local MudCombat = require_module("MudCombat")   -- 按需取消

-- ===== 全域表格 =====
_G.MyScript = _G.MyScript or {}

-- ===== 狀態 =====
_G.MyScript.state = {
    running = false,
    run_id  = 0,
}

-- ===== Hook 註冊 =====
MudUtils.register_hook("MyScript", function(line, clean_line)
    _G.MyScript.on_server_message(line, clean_line)
end)

function _G.MyScript.on_server_message(line, clean_line)
    if not _G.MyScript.state.running then return end
    -- ... 訊息處理邏輯
end

-- ===== 公開 API =====
function _G.MyScript.start()
    _G.MyScript.state.running = true
    _G.MyScript.state.run_id = MudUtils.get_new_run_id()
    MudUtils.register_quest("MyScript", _G.MyScript.stop)
    MudUtils.start_log("myscript")
    mud.echo("[MyScript] 🚀 啟動")
end

function _G.MyScript.stop()
    _G.MyScript.state.running = false
    MudUtils.stop_log()
    mud.echo("[MyScript] 🛑 停止")
end

function _G.MyScript.reload()
    package.loaded["scripts.myscript"] = nil
    require("scripts.myscript")
    mud.echo("[MyScript] ♻️ 已重新載入")
end

-- ===== 啟動說明 =====
MudUtils.print_script_help("MyScript", "v1.0", "描述", {
    { cmd = "MyScript.start()", desc = "啟動" },
    { cmd = "MyScript.stop()",  desc = "停止" },
})
```

---

## 顯式依賴宣告（必要）

### ❌ 禁止：隱性依賴全域

```lua
-- 假設 _G.MudUtils 已存在 → 若載入順序異常，直接 nil 錯誤
MudUtils.register_hook("MyScript", ...)
```

### ✅ 正確：顯式 require

```lua
local MudUtils = require_module("MudUtils")
MudUtils.register_hook("MyScript", ...)
```

**理由**：
- `require` 有快取，重複呼叫不會重新執行，零效能損耗
- 依賴關係文字化，易於閱讀與維護
- 不依賴載入順序，腳本自給自足

---

## 模組介紹

| 模組 | 功能 |
|------|------|
| `MudUtils` | Hook 管理、計時器、日誌、任務生命週期 |
| `MudNav` | 路徑行走、體力恢復、走路回呼 |
| `MudCombat` | 安全召喚（`safe_summon`）、多次重試 |
| `MudExplorer` | DFS/BFS 探索、目標搜尋 |
| `MudLoot` | 戰利品收集、自動 `get` |
| `MudMapper` | 地圖記錄、廣度優先探索 |

---

## Hook 機制

所有伺服器訊息統一由 `MudUtils` 的 dispatcher 分發。腳本**必須**透過 `register_hook` 訂閱，而非直接複寫 `_G.on_server_message`（複寫會覆蓋其他腳本的 hook）。

```lua
-- ✅ 正確
MudUtils.register_hook("MyScript", function(line, clean_line)
    _G.MyScript.on_server_message(line, clean_line)
end)

-- ❌ 禁止
_G.on_server_message = function(line, clean_line)
    -- 這會清除所有其他腳本的 hook！
end
```

### 重新載入後 hook 不疊加

`register_hook` 以名稱為 key，重複呼叫會覆蓋舊 hook，不會巢狀疊加。這是 `reload()` 安全的原因。

---

## 全域命名空間規範

- 每個腳本使用唯一的大寫命名空間：`_G.MyScript`
- 初始化時使用 `or {}`，確保 reload 不清除現有狀態（除非刻意重置）

```lua
_G.MyScript = _G.MyScript or {}
_G.MyScript.state = _G.MyScript.state or { running = false }
```

---

## Timer 與 run_id 防競態

腳本啟動時呼叫 `MudUtils.get_new_run_id()` 取得一個遞增的 ID。所有計時器回呼的第一件事就是驗證 `run_id`，確保舊計時器不會干擾新的執行週期。

```lua
function _G.MyScript.start()
    local rid = MudUtils.get_new_run_id()
    _G.MyScript.state.run_id = rid

    MudUtils.safe_timer(2.0, function(new_rid)
        if new_rid ~= _G.MyScript.state.run_id then return end -- 防舊 timer
        -- ... 執行邏輯
    end)
end
```

> `MudUtils.safe_timer` 會自動封裝 `run_id`，推薦優先使用。

---

## reload() 標準實作

```lua
function _G.MyScript.reload()
    package.loaded["scripts.myscript"] = nil  -- 清除 require 快取
    require("scripts.myscript")               -- 重新執行腳本
    mud.echo("[MyScript] ♻️ 已重新載入")
end
```

注意：模組腳本（`scripts/modules/`）的 reload key 格式為 `"scripts.modules.MudUtils"`。

---

## 腳本載入流程一覽

```
應用程式啟動
   │
   ▼
Rust: load_startup_scripts()
   │
   ├── Phase 1: scripts/modules/*.lua（字母排序）
   │     MudCombat → MudExplorer → MudLoot → MudMapper → MudNav → MudUtils
   │     └── MudUtils 設定 _G.on_server_message dispatcher
   │
   └── Phase 2: scripts/*.lua（字母排序）
         autocast → benumb → combat → help → ikkoku_quest → itemfarm
         → memcalc → mob_finder → poker_quest → practice → ...
         └── 每個腳本透過 require_module() 取到已快取的模組
```

> Modules 層只執行一次，後續的 `require` 直接命中快取（`package.loaded`），零開銷。
