-- ============================================================
-- ChatLog - 即時收集玩家對話到 chat 子視窗
-- ============================================================
-- 用法：require("scripts.chatlog")
-- 自動註冊 trigger，所有玩家對話聚合到 chat 子視窗
-- 訊息同時保留在主視窗，不影響原有顯示
-- ============================================================

_G.ChatLog = _G.ChatLog or {}

local TRIGGER_GROUP = "chatlog"
local TRIGGER_PREFIX = "chatlog_"

local TRIGGERS = {
    { name = "say",      pattern = [[([A-Z][A-Za-z]+) (說|有點開玩笑地說|疑惑地說|高聲地說|面帶微笑地說|嘆了口氣道|嘟囔地說|長長地嘆道|搖了搖頭地說|小聲地說|點了點頭地說|微微一笑地說|冷笑地說|語帶惋惜地說|得意洋洋地說|哈哈大笑地說) ']] },
    { name = "chat",     pattern = [[([A-Z][A-Za-z]+) 談道 ']] },
    { name = "tell",     pattern = [[([A-Z][A-Za-z]+) tell你]] },
    { name = "whisper",  pattern = [[([A-Z][A-Za-z]+) (偷偷告訴你|偷偷回答你)]] },
    { name = "gtell",    pattern = [[([A-Z][A-Za-z]+) 傳來意念]] },
    { name = "my_chat",  pattern = [[你%(.+%) (chat|談道)]] },
    { name = "my_say",   pattern = [[你(說|談道) ']] },
    { name = "my_tell",  pattern = [[你 (tell|reply) ([A-Z][A-Za-z]+)]] },
    { name = "give_in",  pattern = [[([A-Z][A-Za-z]+) 把.+給了你]] },
    { name = "give_out", pattern = [[你把.+給了 ([A-Z][A-Za-z]+)]] },
}

function _G.ChatLog.collect(tag)
    local line = clean_message or ""
    if line == "" then return end

    local ts = os.date("%H:%M")
    mud.window("chat", string.format("[%s] %s", ts, line))
end

function _G.ChatLog.start()
    _G.ChatLog.stop()

    for _, def in ipairs(TRIGGERS) do
        local tname = TRIGGER_PREFIX .. def.name
        mud.add_trigger({
            name = tname,
            pattern = def.pattern,
            pattern_type = "regex",
            script = string.format('ChatLog.collect("%s")', def.name),
            group = TRIGGER_GROUP,
        })
    end
    mud.window("chat", string.format("[%s] [ChatLog] 開始收集玩家對話", os.date("%H:%M")))
    mud.echo("[ChatLog] 已啟動，對話收集到 chat 視窗")
end

function _G.ChatLog.stop()
    for _, def in ipairs(TRIGGERS) do
        mud.remove_trigger(TRIGGER_PREFIX .. def.name)
    end
end

function _G.ChatLog.reload()
    package.loaded["scripts.chatlog"] = nil
    require("scripts.chatlog")
end

-- 自動啟動
_G.ChatLog.start()
