-- Example: Using the recorded Poker Kingdom path
local MudNav = require("scripts.modules.MudNav")

local poker_path = {
    {cmd="s", id="ce9ca433ae6bf286bb394cb20797788aa82752ce41c23db9ac8adf1090d1841a0"},
    {cmd="s", id="c4cc194ee9b17a4babe56a7d3fd09f6b91d12c53815b2f41a2374d91e55cab7d5"},
    {cmd="s", id="c50e9b14484e851fe5f89c254b0d7323d801812d9bb642b60051e369406f612f4"},
    {cmd="s", id="cd8253dcc9aea4a71b5d879ef074cd3f1c5088329ce2c144bb9b6e4c0cec6e618"},
    {cmd="e", id="c341107bbc2a2ce7862c26903d44750da6380667b1cb2b0358a9c453b085e45ff"},
    {cmd="e", id="cf2260278c72687e26cf1ac6a8c4ed66c47e43eb35602296098276e0f82fccf15"},
    {cmd="u", id="c52250d8b7fa01f3399c655689e69ac5ac309dc0b9b566832335277d7f372e82a"},
    {cmd="u", id="c52250d8b7fa01f3399c655689e69ac5ac309dc0b9b566832335277d7f372e82a"},
    {cmd="u", id="c3042cd4b665e86ad0a464868444932c797406b3d281bc58839a46679814801a9"},
    {cmd="u", id="cb85bde0b8311050d5230c05959bb91b6a26b98f30a653b62b127d61051ae9a96"},
}

function go_poker()
    mud.echo("🚀 正在啟動導航至 Poker 王國...")
    MudNav.walk(poker_path, function(success)
        if success then
            mud.echo("✅ 已抵達 Poker 王國入口！")
        else
            mud.echo("🛑 導航失敗：路徑偏移或中斷。")
        end
    end)
end

-- 您也可以將此路徑註冊為全域變數或放在任務腳本中
_G.go_poker = go_poker
