-- Event-Driven Architecture Demo
-- 展示事件系統、觸發器群組、狀態機的整合使用

-- === Event Handlers ===
mud.on("room_changed", "mud.echo('[Event] Room: ' .. (event_data or 'unknown'))")
mud.on("connected", "mud.echo('[Event] Connected!')")

-- === Message Routing ===
-- Route chat messages to a dedicated window
mud.add_route({ name = "chat_route", pattern = "【閒聊】", window = "chat", gag = false })
mud.add_route({ name = "sys_route", pattern = "\\[System\\]", window = "system", gag = false })

-- === State Machine: Grinder Bot ===
mud.state_machine("grinder", {
    initial = "idle",
    states = {
        idle = {
            enter = "mud.enable_group('combat', false); mud.echo('[Bot] Idle mode')",
        },
        fighting = {
            enter = "mud.enable_group('combat', true); mud.echo('[Bot] Combat mode!')",
            exit = "mud.enable_group('combat', false)",
            timeout = { seconds = 120, target = "idle" },
        },
        looting = {
            enter = "mud.send('get all from corpse'); mud.echo('[Bot] Looting...')",
            timeout = { seconds = 10, target = "idle" },
        },
    },
    transitions = {
        { from = "idle",     event = "combat_start", to = "fighting" },
        { from = "fighting", event = "combat_end",   to = "looting" },
        { from = "looting",  event = "loot_done",    to = "idle" },
    },
})

-- === Key Bindings ===
mud.bind_key("f5", "mud.sm_reset('grinder'); mud.echo('[Key] Bot reset!')")
mud.bind_key("f6", "mud.echo('[Key] State: ' .. (mud.sm_current('grinder') or 'none'))")
mud.bind_key("f7", "mud.emit('combat_start')")
mud.bind_key("f8", "mud.emit('combat_end')")

mud.echo("[event_demo] Loaded! F5=reset F6=status F7=start_combat F8=end_combat")
