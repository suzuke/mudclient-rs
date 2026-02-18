-- 自動載入 MudMapper 模組
-- 這樣您就不用每次手動 require 了

local status, err = pcall(require, "modules.MudMapper")
if status then
    mud.echo("{g[System] MudMapper 模組已就緒。{x}")
else
    mud.echo("{r[System] MudMapper 載入失敗: " .. tostring(err) .. "{x}")
end
