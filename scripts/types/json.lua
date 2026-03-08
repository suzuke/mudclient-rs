---@meta

-- mudclient-rs 內建 JSON 模組型別定義
-- 此檔案僅供 LuaLS 使用，不會被執行

---@class json
json = {}

--- 將 Lua 值編碼為 JSON 字串
---
--- 範例：
--- ```lua
--- json.encode({name = "武器", damage = 50})
--- -- => '{"damage":50,"name":"武器"}'
---
--- json.encode({1, 2, 3}, true)
--- -- => '[\n  1,\n  2,\n  3\n]' (pretty print)
--- ```
---@param value any 要編碼的值（table, string, number, boolean, nil）
---@param pretty? boolean 是否美化輸出（2-space indent）
---@return string json_string
function json.encode(value, pretty) end

--- 將 JSON 字串解碼為 Lua 值
---
--- 範例：
--- ```lua
--- local data = json.decode('{"hp":100,"items":["sword","shield"]}')
--- print(data.hp)       -- 100
--- print(data.items[1])  -- "sword"
--- ```
---@param str string JSON 字串
---@return any value 解碼後的 Lua 值（JSON null 對應 nil）
function json.decode(str) end
