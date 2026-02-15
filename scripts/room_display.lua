-- Room Display Hook
-- 當 Rust 偵測到房間時會呼叫此函數
-- id: 房間雜湊 ID
-- name: 房間名稱

function on_room_detected(id, name)
    -- 您可以在這裡自定義顯示格式
    -- 例如: 顯示 ID 在名稱下方，或者顯示為系統訊息
    
    -- 範例 1: 簡單顯示 (類似之前的 debug 訊息)
    -- mud.echo(string.format("{g[System] Room Detected: %s (ID: %s){x}", name, id))
    
    -- 範例 2: 近似使用者要求的格式
    -- 顯示在下一行，且帶有顏色
    mud.echo(string.format("(ID: {c%s{x})", id))
end
