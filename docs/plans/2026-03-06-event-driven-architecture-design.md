# Event-Driven Architecture Upgrade Design

Date: 2026-03-06
Status: Approved

## Overview

為 mudclient-rs 建立事件驅動架構，涵蓋事件系統、觸發器群組、狀態機框架、快捷鍵綁定、訊息分流、腳本 Debug 面板。伺服器不支援 GMCP，所有事件基於文字解析和客戶端狀態。

## 1. Event Bus

在 `mudcore` 層新增 `EventBus`，所有事件統一流經。

### Event Sources (3 types)

**Client events** (Rust auto-generated):
- `connected`, `disconnected`, `reconnecting`
- `room_changed(old_id, new_id, room_name)`
- `command_sent(cmd)`
- `state_changed(old_state, new_state)`

**Text-parsed events** (via event triggers):
- `combat_start`, `combat_end`
- `chat_message(channel, sender, text)`
- `char_vitals(hp, mp, vigor)`
- User-defined custom events

**Timer events**: `timer_fired(timer_id)`

### Lua API

```lua
local id = mud.on("combat_end", function(event)
    mud.send("get all from corpse")
end)
mud.off(id)
mud.emit("custom_event", { key = "value" })
mud.once("connected", function() mud.send("look") end)
```

### Rust Implementation

`EventBus` holds `HashMap<String, Vec<EventHandler>>`. Each handler: Lua callback + priority + enabled flag. On emit, execute handlers in priority order.

## 2. Trigger Groups

Each trigger can belong to a group. Groups can be toggled as a unit.

```lua
mud.enable_group("combat_triggers", true)
mud.enable_group("combat_triggers", false)

mud.add_trigger({
    name = "auto_heal",
    pattern = "...",
    group = "combat_triggers",
    action = function(line) mud.send("heal") end
})
```

State machine transitions can auto-toggle groups.

## 3. State Machine Framework

```lua
local sm = mud.state_machine("quest_bot", {
    initial = "idle",
    states = {
        idle = {
            enter = function() mud.enable_group("idle_triggers", true) end,
            exit  = function() mud.enable_group("idle_triggers", false) end,
        },
        fighting = {
            enter = function() mud.enable_group("combat_triggers", true) end,
            exit  = function() mud.enable_group("combat_triggers", false) end,
            timeout = { seconds = 60, goto = "idle" },
        },
        looting = {
            enter = function() mud.send("get all from corpse") end,
            timeout = { seconds = 10, goto = "idle" },
        },
        fleeing = {
            enter = function() mud.send("flee") end,
        },
    },
    transitions = {
        { from = "idle",     event = "combat_start", to = "fighting" },
        { from = "fighting", event = "combat_end",   to = "looting" },
        { from = "fighting", event = "low_hp",       to = "fleeing" },
        { from = "looting",  event = "loot_done",    to = "idle" },
        { from = "fleeing",  event = "room_changed",  to = "idle" },
    },
})

sm:current()                    --> "idle"
sm:transition("combat_start")   -- manual trigger
sm:reset()                      -- back to initial
```

### Rust Implementation

`StateMachine` struct in `ScriptEngine`. Named instances. On event: lookup transition table -> execute exit callback -> switch state -> execute enter callback. Timeouts via existing timer mechanism.

## 4. Key Bindings

```lua
mud.bind_key("f1", "look")
mud.bind_key("ctrl+1", function() mud.send("report") end)
mud.unbind_key("f1")
```

Intercept in `app/mod.rs` input handling. `HashMap<KeyCombo, LuaCallback>`. Supports F1-F12, Ctrl+0-9, Ctrl+letter.

## 5. Message Routing

Route messages to sub-windows based on events or patterns:

```lua
mud.route("chat_message", "chat")

mud.add_route({
    pattern = "...",
    window = "chat",
    gag = false,  -- true = hide from main buffer
})
```

In trigger processing pipeline, matched lines are copied to target sub-window. `gag = true` removes from main buffer.

## 6. Script Debug Panel

- Sidebar **Lua Console** tab for interactive eval
- **Variable inspector** showing `mud.vars` and state machine states
- **Event log** showing recent N events fired

GUI-only work, no core architecture impact.

## Data Flow

```
MUD Server -> TCP -> Telnet Parser -> Raw Text
                                        |
                                Trigger Engine --> match "event triggers"
                                        |                |
                                Main Buffer        EventBus.emit(event)
                                        |                |
                                Route Rules    +-- Event Handlers (Lua callbacks)
                                    |          +-- State Machine transitions
                                Sub-Windows    +-- Trigger Group switches
```

## Design Decisions

- No GMCP: server (void7777.ddns.net:7777) confirmed not supporting GMCP
- Events are string-keyed with optional Lua table payload
- State machines are named instances, allowing multiple concurrent machines
- Trigger groups are additive (a trigger belongs to at most one group; ungrouped triggers always active)
- Key bindings are session-scoped, stored in profile config
