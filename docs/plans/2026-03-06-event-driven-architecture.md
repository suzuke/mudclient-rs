# Event-Driven Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add event bus, trigger groups, state machine framework, key bindings, message routing, and script debug panel to mudclient-rs.

**Architecture:** New `event.rs` module in mudcore holds EventBus. ScriptEngine gains mud.on/off/emit/once/state_machine/bind_key APIs. Session emits client events. GUI adds debug panel to sidebar.

**Tech Stack:** Rust, mlua (Lua 5.4), egui, tokio

---

## Task 1: EventBus Core (mudcore)

**Files:**
- Create: `crates/mudcore/src/event.rs`
- Modify: `crates/mudcore/src/lib.rs`

### Step 1: Create event.rs with EventBus struct

```rust
// crates/mudcore/src/event.rs
use std::collections::HashMap;

/// Unique handler ID for registration/deregistration
pub type HandlerId = u64;

/// Event payload: name + optional data as JSON string
#[derive(Debug, Clone)]
pub struct Event {
    pub name: String,
    pub data: Option<String>, // JSON string from Lua table
}

/// Single event handler registration
#[derive(Debug, Clone)]
pub struct EventHandler {
    pub id: HandlerId,
    pub lua_code: String,
    pub priority: i32,
    pub once: bool,
    pub enabled: bool,
}

/// Central event bus for the MUD client
pub struct EventBus {
    handlers: HashMap<String, Vec<EventHandler>>,
    next_id: HandlerId,
    /// Events emitted during current processing cycle, collected by caller
    pub pending_events: Vec<Event>,
    /// Recent event log for debug panel
    pub event_log: Vec<(std::time::Instant, String, Option<String>)>,
    event_log_capacity: usize,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            next_id: 1,
            pending_events: Vec::new(),
            event_log: Vec::new(),
            event_log_capacity: 100,
        }
    }

    /// Register a handler. Returns handler ID for later removal.
    pub fn on(&mut self, event_name: &str, lua_code: String, priority: i32, once: bool) -> HandlerId {
        let id = self.next_id;
        self.next_id += 1;
        let handler = EventHandler {
            id,
            lua_code,
            priority,
            once,
            enabled: true,
        };
        let handlers = self.handlers.entry(event_name.to_string()).or_default();
        handlers.push(handler);
        handlers.sort_by_key(|h| h.priority);
        id
    }

    /// Remove a handler by ID. Returns true if found.
    pub fn off(&mut self, handler_id: HandlerId) -> bool {
        for handlers in self.handlers.values_mut() {
            if let Some(pos) = handlers.iter().position(|h| h.id == handler_id) {
                handlers.remove(pos);
                return true;
            }
        }
        false
    }

    /// Queue an event for emission. Returns handlers to execute.
    /// Caller is responsible for executing the Lua code.
    pub fn emit(&mut self, event_name: &str, data: Option<String>) -> Vec<(HandlerId, String)> {
        // Log the event
        self.event_log.push((std::time::Instant::now(), event_name.to_string(), data.clone()));
        if self.event_log.len() > self.event_log_capacity {
            self.event_log.remove(0);
        }

        let mut to_execute = Vec::new();
        let mut to_remove = Vec::new();

        if let Some(handlers) = self.handlers.get(event_name) {
            for handler in handlers {
                if handler.enabled {
                    to_execute.push((handler.id, handler.lua_code.clone()));
                    if handler.once {
                        to_remove.push(handler.id);
                    }
                }
            }
        }

        // Remove once handlers
        for id in to_remove {
            self.off(id);
        }

        to_execute
    }

    /// Get all registered event names
    pub fn registered_events(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Get handler count for an event
    pub fn handler_count(&self, event_name: &str) -> usize {
        self.handlers.get(event_name).map_or(0, |h| h.len())
    }

    /// Clear all handlers
    pub fn clear(&mut self) {
        self.handlers.clear();
    }
}
```

### Step 2: Register module in lib.rs

Add to `crates/mudcore/src/lib.rs`:

```rust
pub mod event;
pub use event::EventBus;
```

### Step 3: Add unit tests to event.rs

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_off() {
        let mut bus = EventBus::new();
        let id = bus.on("test", "print('hi')".into(), 0, false);
        assert_eq!(bus.handler_count("test"), 1);
        assert!(bus.off(id));
        assert_eq!(bus.handler_count("test"), 0);
    }

    #[test]
    fn test_emit_returns_handlers() {
        let mut bus = EventBus::new();
        bus.on("combat_end", "mud.send('loot')".into(), 0, false);
        bus.on("combat_end", "mud.echo('done')".into(), 10, false);
        let handlers = bus.emit("combat_end", None);
        assert_eq!(handlers.len(), 2);
        // Priority order: 0 first, 10 second
        assert!(handlers[0].1.contains("loot"));
        assert!(handlers[1].1.contains("done"));
    }

    #[test]
    fn test_once_handler_removed_after_emit() {
        let mut bus = EventBus::new();
        bus.on("connected", "mud.send('look')".into(), 0, true);
        let handlers = bus.emit("connected", None);
        assert_eq!(handlers.len(), 1);
        // Second emit should return nothing
        let handlers = bus.emit("connected", None);
        assert_eq!(handlers.len(), 0);
    }

    #[test]
    fn test_event_log() {
        let mut bus = EventBus::new();
        bus.emit("room_changed", Some(r#"{"id":"abc"}"#.into()));
        assert_eq!(bus.event_log.len(), 1);
        assert_eq!(bus.event_log[0].1, "room_changed");
    }

    #[test]
    fn test_priority_ordering() {
        let mut bus = EventBus::new();
        bus.on("test", "second".into(), 10, false);
        bus.on("test", "first".into(), 0, false);
        let handlers = bus.emit("test", None);
        assert_eq!(handlers[0].1, "first");
        assert_eq!(handlers[1].1, "second");
    }
}
```

### Step 4: Run tests

```bash
cd crates/mudcore && cargo test event::tests
```

### Step 5: Commit

```bash
git add crates/mudcore/src/event.rs crates/mudcore/src/lib.rs
git commit -m "feat: add EventBus core module with on/off/emit/once support"
```

---

## Task 2: Lua Event API (mud.on/off/emit/once)

**Files:**
- Modify: `crates/mudcore/src/script.rs` (MudContext, API registration)
- Modify: `crates/mudcore/src/event.rs` (if needed)

### Step 1: Add event fields to MudContext

In `crates/mudcore/src/script.rs`, add to `MudContext` struct (around line 31):

```rust
/// Event handler registrations: (event_name, lua_code, priority, once)
pub event_registrations: Vec<(String, String, i32, bool)>,
/// Event handler removals: handler_id
pub event_removals: Vec<u64>,
/// Events to emit: (event_name, data_json)
pub event_emissions: Vec<(String, Option<String>)>,
```

Initialize them as empty Vecs in `run_code` where MudContext is built.

### Step 2: Register mud.on/off/emit/once in Lua scope

In the `run_code` method (around line 253), add Lua table setup:

```rust
// Event registration table
mud_table.set("_event_registrations", lua.create_table()?)?;
mud_table.set("_event_removals", lua.create_table()?)?;
mud_table.set("_event_emissions", lua.create_table()?)?;
```

Add Lua functions:

```rust
// mud.on(event_name, callback_code, [priority]) -> handler_id
// Returns a placeholder ID; real ID assigned by EventBus
let on_fn = lua.create_function(|lua_ctx, (event_name, code, priority): (String, String, Option<i32>)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let regs: mlua::Table = mud.get("_event_registrations")?;
    let len = regs.len()? + 1;
    let entry = lua_ctx.create_table()?;
    entry.set(1, event_name)?;
    entry.set(2, code)?;
    entry.set(3, priority.unwrap_or(0))?;
    entry.set(4, false)?; // once = false
    regs.set(len, entry)?;
    Ok(len) // placeholder ID
})?;
mud_table.set("on", on_fn)?;

// mud.once(event_name, callback_code, [priority]) -> handler_id
let once_fn = lua.create_function(|lua_ctx, (event_name, code, priority): (String, String, Option<i32>)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let regs: mlua::Table = mud.get("_event_registrations")?;
    let len = regs.len()? + 1;
    let entry = lua_ctx.create_table()?;
    entry.set(1, event_name)?;
    entry.set(2, code)?;
    entry.set(3, priority.unwrap_or(0))?;
    entry.set(4, true)?; // once = true
    regs.set(len, entry)?;
    Ok(len)
})?;
mud_table.set("once", once_fn)?;

// mud.off(handler_id)
let off_fn = lua.create_function(|lua_ctx, handler_id: u64| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let removals: mlua::Table = mud.get("_event_removals")?;
    let len = removals.len()? + 1;
    removals.set(len, handler_id)?;
    Ok(())
})?;
mud_table.set("off", off_fn)?;

// mud.emit(event_name, [data_table])
let emit_fn = lua.create_function(|lua_ctx, (event_name, data): (String, Option<mlua::Value>)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let emissions: mlua::Table = mud.get("_event_emissions")?;
    let len = emissions.len()? + 1;
    let entry = lua_ctx.create_table()?;
    entry.set(1, event_name)?;
    // Convert Lua table to JSON string if provided
    let json_str = match data {
        Some(mlua::Value::Table(t)) => {
            // Simple table-to-JSON: use serde_json via mlua
            let val: serde_json::Value = lua_ctx.from_value(mlua::Value::Table(t))?;
            Some(serde_json::to_string(&val).unwrap_or_default())
        }
        Some(mlua::Value::String(s)) => Some(s.to_str()?.to_string()),
        _ => None,
    };
    entry.set(2, json_str)?;
    emissions.set(len, entry)?;
    Ok(())
})?;
mud_table.set("emit", emit_fn)?;
```

### Step 3: Collect event data into MudContext

In the MudContext collection section (around line 571-687), add:

```rust
// Collect event registrations
if let Ok(regs) = mud_table.get::<mlua::Table>("_event_registrations") {
    for i in 1..=regs.len().unwrap_or(0) {
        if let Ok(entry) = regs.get::<mlua::Table>(i) {
            if let (Ok(name), Ok(code), Ok(priority), Ok(once)) = (
                entry.get::<String>(1),
                entry.get::<String>(2),
                entry.get::<i32>(3),
                entry.get::<bool>(4),
            ) {
                context.event_registrations.push((name, code, priority, once));
            }
        }
    }
}

// Collect event removals
if let Ok(removals) = mud_table.get::<mlua::Table>("_event_removals") {
    for i in 1..=removals.len().unwrap_or(0) {
        if let Ok(id) = removals.get::<u64>(i) {
            context.event_removals.push(id);
        }
    }
}

// Collect event emissions
if let Ok(emissions) = mud_table.get::<mlua::Table>("_event_emissions") {
    for i in 1..=emissions.len().unwrap_or(0) {
        if let Ok(entry) = emissions.get::<mlua::Table>(i) {
            if let Ok(name) = entry.get::<String>(1) {
                let data = entry.get::<Option<String>>(2).unwrap_or(None);
                context.event_emissions.push((name, data));
            }
        }
    }
}
```

### Step 4: Run tests

```bash
cd crates/mudcore && cargo test
```

### Step 5: Commit

```bash
git add crates/mudcore/src/script.rs
git commit -m "feat: add mud.on/off/emit/once Lua API for event system"
```

---

## Task 3: Wire EventBus into Session

**Files:**
- Modify: `crates/mudgui/src/session.rs` (add EventBus field, process events in apply_script_context, emit client events)

### Step 1: Add EventBus to Session

In Session struct (around line 131), add:

```rust
pub event_bus: EventBus,
```

Initialize in `from_profile()` (around line 407):

```rust
event_bus: EventBus::new(),
```

### Step 2: Process event data in apply_script_context

In `apply_script_context()` (around line 1194), add handling for event fields:

```rust
// Process event registrations
for (name, code, priority, once) in context.event_registrations {
    self.event_bus.on(&name, code, priority, once);
}

// Process event removals
for id in context.event_removals {
    self.event_bus.off(id);
}

// Process event emissions
for (name, data) in context.event_emissions {
    let handlers = self.event_bus.emit(&name, data.clone());
    for (_id, lua_code) in handlers {
        // Set event data as global before executing
        let setup = if let Some(ref d) = data {
            format!("event_data = '{}'", d.replace('\'', "\\'"))
        } else {
            "event_data = nil".to_string()
        };
        let full_code = format!("{}\n{}", setup, lua_code);
        if let Ok(ctx) = self.script_engine.execute_inline(&full_code, "", &[], false) {
            self.apply_script_context(ctx);
        }
    }
}
```

### Step 3: Emit built-in client events

Add event emissions at key points in session.rs:

**connected** — in `start_connection()` callback or status change (app/mod.rs):
```rust
// After connection established
self.event_bus.emit("connected", None);
```

**disconnected** — when connection drops:
```rust
self.event_bus.emit("disconnected", None);
```

**room_changed** — in `detect_room_info()` (around line 1050):
```rust
// After current_room_id is updated
let data = serde_json::json!({
    "id": new_room_id,
    "name": room_name,
}).to_string();
let handlers = self.event_bus.emit("room_changed", Some(data));
// Execute handlers...
```

**command_sent** — in `handle_user_input_with_depth()` before sending to MUD:
```rust
let handlers = self.event_bus.emit("command_sent", Some(
    serde_json::json!({"command": &input}).to_string()
));
```

### Step 4: Run full build

```bash
cargo build 2>&1 | head -30
```

### Step 5: Commit

```bash
git add crates/mudgui/src/session.rs
git commit -m "feat: wire EventBus into Session with client event emission"
```

---

## Task 4: Trigger Groups

**Files:**
- Modify: `crates/mudcore/src/trigger.rs` (add group field, enable_group method)
- Modify: `crates/mudcore/src/script.rs` (add mud.enable_group API)
- Modify: `crates/mudgui/src/config.rs` (add group to TriggerConfig)

### Step 1: Add group field to Trigger

In `crates/mudcore/src/trigger.rs`, Trigger struct (line 40):

```rust
pub struct Trigger {
    pub name: String,
    pub category: Option<String>,
    pub group: Option<String>,  // NEW: trigger group for batch enable/disable
    pub pattern: TriggerPattern,
    pub actions: Vec<TriggerAction>,
    pub enabled: bool,
    compiled_regex: Option<Regex>,
}
```

Update `Trigger::new()` to initialize `group: None`.

Add builder method:
```rust
pub fn with_group(mut self, group: impl Into<String>) -> Self {
    self.group = Some(group.into());
    self
}
```

### Step 2: Add enable_group to TriggerManager

```rust
/// Enable or disable all triggers in a group. Returns count of affected triggers.
pub fn enable_group(&mut self, group: &str, enabled: bool) -> usize {
    let mut count = 0;
    for trigger in self.triggers.values_mut() {
        if trigger.group.as_deref() == Some(group) {
            trigger.enabled = enabled;
            count += 1;
        }
    }
    count
}

/// List all unique group names
pub fn groups(&self) -> Vec<String> {
    let mut groups: Vec<String> = self.triggers.values()
        .filter_map(|t| t.group.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    groups.sort();
    groups
}
```

### Step 3: Add group to TriggerConfig

In `crates/mudgui/src/config.rs`, TriggerConfig (line 49):

```rust
pub struct TriggerConfig {
    pub name: String,
    pub pattern: String,
    pub action: String,
    pub category: Option<String>,
    pub group: Option<String>,  // NEW
    pub is_script: bool,
    pub enabled: bool,
    pub pattern_type: Option<String>,
}
```

Update all TriggerConfig construction sites to include `group: None` or propagate from Trigger.

### Step 4: Add mud.enable_group to Lua API

In `crates/mudcore/src/script.rs`, add to MudContext:

```rust
pub group_updates: Vec<(String, bool)>,  // (group_name, enabled)
```

Add Lua function:

```rust
// mud.enable_group(group_name, enabled)
let enable_group_fn = lua.create_function(|lua_ctx, (group, enabled): (String, bool)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let updates: mlua::Table = mud.get("_group_updates")?;
    let len = updates.len()? + 1;
    let entry = lua_ctx.create_table()?;
    entry.set(1, group)?;
    entry.set(2, enabled)?;
    updates.set(len, entry)?;
    Ok(())
})?;
mud_table.set("enable_group", enable_group_fn)?;
```

Process in apply_script_context:

```rust
for (group, enabled) in context.group_updates {
    self.trigger_manager.enable_group(&group, enabled);
}
```

### Step 5: Add tests

In `crates/mudcore/src/trigger.rs` tests:

```rust
#[test]
fn test_trigger_groups() {
    let mut mgr = TriggerManager::new();
    mgr.add(Trigger::new("t1", TriggerPattern::Contains("a".into()), vec![])
        .with_group("combat"));
    mgr.add(Trigger::new("t2", TriggerPattern::Contains("b".into()), vec![])
        .with_group("combat"));
    mgr.add(Trigger::new("t3", TriggerPattern::Contains("c".into()), vec![]));

    assert_eq!(mgr.enable_group("combat", false), 2);
    assert!(!mgr.get("t1").unwrap().enabled);
    assert!(!mgr.get("t2").unwrap().enabled);
    assert!(mgr.get("t3").unwrap().enabled); // ungrouped unaffected

    assert_eq!(mgr.enable_group("combat", true), 2);
    assert!(mgr.get("t1").unwrap().enabled);
}

#[test]
fn test_groups_list() {
    let mut mgr = TriggerManager::new();
    mgr.add(Trigger::new("t1", TriggerPattern::Contains("a".into()), vec![])
        .with_group("combat"));
    mgr.add(Trigger::new("t2", TriggerPattern::Contains("b".into()), vec![])
        .with_group("explore"));
    let groups = mgr.groups();
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&"combat".to_string()));
    assert!(groups.contains(&"explore".to_string()));
}
```

### Step 6: Run tests

```bash
cd crates/mudcore && cargo test trigger::tests
```

### Step 7: Commit

```bash
git add crates/mudcore/src/trigger.rs crates/mudcore/src/script.rs crates/mudgui/src/config.rs
git commit -m "feat: add trigger groups with mud.enable_group() API"
```

---

## Task 5: State Machine Framework

**Files:**
- Create: `crates/mudcore/src/state_machine.rs`
- Modify: `crates/mudcore/src/lib.rs`
- Modify: `crates/mudcore/src/script.rs` (add mud.state_machine API)
- Modify: `crates/mudgui/src/session.rs` (integrate state machines with event bus)

### Step 1: Create state_machine.rs

```rust
// crates/mudcore/src/state_machine.rs
use std::collections::HashMap;

/// A single state definition
#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub enter_code: Option<String>,  // Lua code on enter
    pub exit_code: Option<String>,   // Lua code on exit
    pub timeout_secs: Option<f64>,
    pub timeout_goto: Option<String>,
}

/// A transition rule
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub event: String,
    pub to: String,
    pub guard_code: Option<String>,  // Optional Lua condition
}

/// Result of a state transition
#[derive(Debug)]
pub struct TransitionResult {
    pub exit_code: Option<String>,
    pub enter_code: Option<String>,
    pub old_state: String,
    pub new_state: String,
}

/// A named state machine instance
pub struct StateMachine {
    pub name: String,
    pub current: String,
    pub initial: String,
    pub states: HashMap<String, State>,
    pub transitions: Vec<Transition>,
    /// When current state's timeout expires (if any)
    pub timeout_at: Option<std::time::Instant>,
}

impl StateMachine {
    pub fn new(name: String, initial: String, states: HashMap<String, State>, transitions: Vec<Transition>) -> Self {
        let timeout_at = states.get(&initial)
            .and_then(|s| s.timeout_secs)
            .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs_f64(secs));

        Self {
            name,
            current: initial.clone(),
            initial,
            states,
            transitions,
            timeout_at,
        }
    }

    /// Try to transition on an event. Returns transition result if successful.
    pub fn handle_event(&mut self, event: &str) -> Option<TransitionResult> {
        let transition = self.transitions.iter()
            .find(|t| t.from == self.current && t.event == event)?;

        let to_state = transition.to.clone();
        self.do_transition(to_state)
    }

    /// Check timeout. Returns transition result if timed out.
    pub fn check_timeout(&mut self) -> Option<TransitionResult> {
        if let Some(timeout_at) = self.timeout_at {
            if std::time::Instant::now() >= timeout_at {
                let goto = self.states.get(&self.current)
                    .and_then(|s| s.timeout_goto.clone())?;
                return self.do_transition(goto);
            }
        }
        None
    }

    /// Reset to initial state
    pub fn reset(&mut self) -> Option<TransitionResult> {
        let initial = self.initial.clone();
        self.do_transition(initial)
    }

    fn do_transition(&mut self, to: String) -> Option<TransitionResult> {
        if !self.states.contains_key(&to) {
            return None;
        }

        let old_state = self.current.clone();
        let exit_code = self.states.get(&old_state).and_then(|s| s.exit_code.clone());
        let enter_code = self.states.get(&to).and_then(|s| s.enter_code.clone());

        self.current = to.clone();

        // Reset timeout for new state
        self.timeout_at = self.states.get(&to)
            .and_then(|s| s.timeout_secs)
            .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs_f64(secs));

        Some(TransitionResult {
            exit_code,
            enter_code,
            old_state,
            new_state: to,
        })
    }

    pub fn current_state(&self) -> &str {
        &self.current
    }
}

/// Manages multiple named state machines
pub struct StateMachineManager {
    pub machines: HashMap<String, StateMachine>,
}

impl StateMachineManager {
    pub fn new() -> Self {
        Self { machines: HashMap::new() }
    }

    pub fn add(&mut self, machine: StateMachine) {
        self.machines.insert(machine.name.clone(), machine);
    }

    pub fn get(&self, name: &str) -> Option<&StateMachine> {
        self.machines.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut StateMachine> {
        self.machines.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<StateMachine> {
        self.machines.remove(name)
    }

    /// Handle event across all machines. Returns list of (machine_name, TransitionResult).
    pub fn handle_event(&mut self, event: &str) -> Vec<(String, TransitionResult)> {
        let mut results = Vec::new();
        for (name, machine) in &mut self.machines {
            if let Some(result) = machine.handle_event(event) {
                results.push((name.clone(), result));
            }
        }
        results
    }

    /// Check timeouts across all machines.
    pub fn check_timeouts(&mut self) -> Vec<(String, TransitionResult)> {
        let mut results = Vec::new();
        for (name, machine) in &mut self.machines {
            if let Some(result) = machine.check_timeout() {
                results.push((name.clone(), result));
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_machine() -> StateMachine {
        let mut states = HashMap::new();
        states.insert("idle".into(), State {
            name: "idle".into(),
            enter_code: Some("mud.echo('entered idle')".into()),
            exit_code: Some("mud.echo('left idle')".into()),
            timeout_secs: None,
            timeout_goto: None,
        });
        states.insert("fighting".into(), State {
            name: "fighting".into(),
            enter_code: Some("mud.echo('combat!')".into()),
            exit_code: None,
            timeout_secs: Some(60.0),
            timeout_goto: Some("idle".into()),
        });
        states.insert("looting".into(), State {
            name: "looting".into(),
            enter_code: Some("mud.send('get all')".into()),
            exit_code: None,
            timeout_secs: Some(10.0),
            timeout_goto: Some("idle".into()),
        });

        let transitions = vec![
            Transition { from: "idle".into(), event: "combat_start".into(), to: "fighting".into(), guard_code: None },
            Transition { from: "fighting".into(), event: "combat_end".into(), to: "looting".into(), guard_code: None },
            Transition { from: "looting".into(), event: "loot_done".into(), to: "idle".into(), guard_code: None },
        ];

        StateMachine::new("test_bot".into(), "idle".into(), states, transitions)
    }

    #[test]
    fn test_basic_transition() {
        let mut sm = make_test_machine();
        assert_eq!(sm.current_state(), "idle");

        let result = sm.handle_event("combat_start").unwrap();
        assert_eq!(result.old_state, "idle");
        assert_eq!(result.new_state, "fighting");
        assert!(result.exit_code.unwrap().contains("left idle"));
        assert!(result.enter_code.unwrap().contains("combat!"));
    }

    #[test]
    fn test_no_matching_transition() {
        let mut sm = make_test_machine();
        assert!(sm.handle_event("combat_end").is_none()); // not valid from idle
    }

    #[test]
    fn test_full_cycle() {
        let mut sm = make_test_machine();
        sm.handle_event("combat_start");
        assert_eq!(sm.current_state(), "fighting");
        sm.handle_event("combat_end");
        assert_eq!(sm.current_state(), "looting");
        sm.handle_event("loot_done");
        assert_eq!(sm.current_state(), "idle");
    }

    #[test]
    fn test_reset() {
        let mut sm = make_test_machine();
        sm.handle_event("combat_start");
        assert_eq!(sm.current_state(), "fighting");
        sm.reset();
        assert_eq!(sm.current_state(), "idle");
    }

    #[test]
    fn test_timeout_set() {
        let mut sm = make_test_machine();
        assert!(sm.timeout_at.is_none()); // idle has no timeout
        sm.handle_event("combat_start");
        assert!(sm.timeout_at.is_some()); // fighting has 60s timeout
    }

    #[test]
    fn test_manager_broadcast_event() {
        let mut mgr = StateMachineManager::new();
        mgr.add(make_test_machine());
        let results = mgr.handle_event("combat_start");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "test_bot");
    }
}
```

### Step 2: Register module in lib.rs

```rust
pub mod state_machine;
pub use state_machine::{StateMachine, StateMachineManager};
```

### Step 3: Add mud.state_machine() Lua API

In `crates/mudcore/src/script.rs`, add to MudContext:

```rust
/// State machine definitions: (name, initial, states_json, transitions_json)
pub state_machine_defs: Vec<(String, String, String, String)>,
/// State machine manual transitions: (machine_name, event_name)
pub state_machine_transitions: Vec<(String, String)>,
/// State machine resets: machine_name
pub state_machine_resets: Vec<String>,
```

Add Lua API. This is the most complex function — it parses the Lua table definition:

```rust
// mud.state_machine(name, definition_table)
// Returns a userdata with :current(), :transition(event), :reset() methods
// The actual StateMachine is created in Rust via apply_script_context
let state_machine_fn = lua.create_function(|lua_ctx, (name, def): (String, mlua::Table)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let defs: mlua::Table = mud.get("_sm_defs")?;
    let len = defs.len()? + 1;

    let initial: String = def.get("initial")?;

    // Serialize states
    let states_table: mlua::Table = def.get("states")?;
    let mut states_json = serde_json::Map::new();
    for pair in states_table.pairs::<String, mlua::Table>() {
        let (state_name, state_def) = pair?;
        let mut state_obj = serde_json::Map::new();
        if let Ok(enter) = state_def.get::<String>("enter") {
            state_obj.insert("enter".into(), serde_json::Value::String(enter));
        }
        if let Ok(exit) = state_def.get::<String>("exit") {
            state_obj.insert("exit".into(), serde_json::Value::String(exit));
        }
        if let Ok(timeout) = state_def.get::<mlua::Table>("timeout") {
            if let (Ok(secs), Ok(goto)) = (timeout.get::<f64>("seconds"), timeout.get::<String>("goto")) {
                state_obj.insert("timeout_secs".into(), serde_json::Value::from(secs));
                state_obj.insert("timeout_goto".into(), serde_json::Value::String(goto));
            }
        }
        states_json.insert(state_name, serde_json::Value::Object(state_obj));
    }

    // Serialize transitions
    let trans_table: mlua::Table = def.get("transitions")?;
    let mut trans_json = Vec::new();
    for i in 1..=trans_table.len()? {
        let t: mlua::Table = trans_table.get(i)?;
        let from: String = t.get("from")?;
        let event: String = t.get("event")?;
        let to: String = t.get("to")?;
        trans_json.push(serde_json::json!({"from": from, "event": event, "to": to}));
    }

    let entry = lua_ctx.create_table()?;
    entry.set(1, name.clone())?;
    entry.set(2, initial)?;
    entry.set(3, serde_json::Value::Object(states_json).to_string())?;
    entry.set(4, serde_json::Value::Array(trans_json).to_string())?;
    defs.set(len, entry)?;

    Ok(name)
})?;
mud_table.set("state_machine", state_machine_fn)?;

// mud.sm_current(name) -> state string
let sm_current_fn = lua.create_function(|lua_ctx, name: String| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let states: mlua::Table = mud.get("_sm_states")?;
    let state: Option<String> = states.get(name)?;
    Ok(state)
})?;
mud_table.set("sm_current", sm_current_fn)?;

// mud.sm_transition(name, event) - manual transition
let sm_transition_fn = lua.create_function(|lua_ctx, (name, event): (String, String)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let transitions: mlua::Table = mud.get("_sm_transitions")?;
    let len = transitions.len()? + 1;
    let entry = lua_ctx.create_table()?;
    entry.set(1, name)?;
    entry.set(2, event)?;
    transitions.set(len, entry)?;
    Ok(())
})?;
mud_table.set("sm_transition", sm_transition_fn)?;

// mud.sm_reset(name)
let sm_reset_fn = lua.create_function(|lua_ctx, name: String| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let resets: mlua::Table = mud.get("_sm_resets")?;
    let len = resets.len()? + 1;
    resets.set(len, name)?;
    Ok(())
})?;
mud_table.set("sm_reset", sm_reset_fn)?;
```

NOTE: The `enter` and `exit` callbacks use string Lua code (not closures) because mlua closures cannot cross scope boundaries. Users write:

```lua
mud.state_machine("bot", {
    initial = "idle",
    states = {
        idle = { enter = "mud.echo('idle now')" },
        fighting = {
            enter = "mud.enable_group('combat', true)",
            exit = "mud.enable_group('combat', false)",
            timeout = { seconds = 60, goto = "idle" },
        },
    },
    transitions = {
        { from = "idle", event = "combat_start", to = "fighting" },
    },
})
```

### Step 4: Integrate state machines in Session

In `crates/mudgui/src/session.rs`, add:

```rust
pub state_machines: StateMachineManager,
```

In `apply_script_context()`, process state machine definitions, transitions, resets.

In `check_timers()`, also call `state_machines.check_timeouts()` and execute resulting callbacks.

When EventBus emits an event, also call `state_machines.handle_event()` and execute `exit_code`/`enter_code`, plus emit `state_changed` event.

### Step 5: Run tests

```bash
cd crates/mudcore && cargo test state_machine::tests
```

### Step 6: Commit

```bash
git add crates/mudcore/src/state_machine.rs crates/mudcore/src/lib.rs crates/mudcore/src/script.rs crates/mudgui/src/session.rs
git commit -m "feat: add state machine framework with mud.state_machine() API"
```

---

## Task 6: Key Bindings

**Files:**
- Modify: `crates/mudcore/src/script.rs` (add mud.bind_key/unbind_key API)
- Modify: `crates/mudgui/src/session.rs` (store key bindings)
- Modify: `crates/mudgui/src/app/mod.rs` (intercept key events)

### Step 1: Add key binding storage to Session

In `crates/mudgui/src/session.rs`:

```rust
/// Key bindings: KeyCombo string -> Lua code
pub key_bindings: HashMap<String, String>,
```

Add to MudContext:

```rust
pub key_bindings: Vec<(String, Option<String>)>,  // (key_combo, Some(code) = bind, None = unbind)
```

### Step 2: Add mud.bind_key/unbind_key Lua API

```rust
// mud.bind_key(key_combo, lua_code_or_command)
let bind_key_fn = lua.create_function(|lua_ctx, (key, code): (String, String)| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let bindings: mlua::Table = mud.get("_key_bindings")?;
    let len = bindings.len()? + 1;
    let entry = lua_ctx.create_table()?;
    entry.set(1, key)?;
    entry.set(2, code)?;
    bindings.set(len, entry)?;
    Ok(())
})?;
mud_table.set("bind_key", bind_key_fn)?;

// mud.unbind_key(key_combo)
let unbind_key_fn = lua.create_function(|lua_ctx, key: String| {
    let mud: mlua::Table = lua_ctx.globals().get("mud")?;
    let bindings: mlua::Table = mud.get("_key_unbindings")?;
    let len = bindings.len()? + 1;
    bindings.set(len, key)?;
    Ok(())
})?;
mud_table.set("unbind_key", unbind_key_fn)?;
```

### Step 3: Intercept in handle_keyboard_shortcuts

In `crates/mudgui/src/app/mod.rs`, `handle_keyboard_shortcuts()` (around line 1527):

Add key binding check before existing shortcut handling:

```rust
// Check Lua key bindings for active session
if let Some(session) = self.session_manager.active_session_mut() {
    let key_str = Self::egui_key_to_string(key, modifiers);
    if let Some(code) = session.key_bindings.get(&key_str).cloned() {
        if let Ok(ctx) = session.script_engine.execute_inline(&code, "", &[], false) {
            session.apply_script_context(ctx);
        }
        // Consume the event
        return true;
    }
}
```

Add helper to convert egui Key + Modifiers to string:

```rust
fn egui_key_to_string(key: egui::Key, modifiers: &egui::Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.ctrl || modifiers.mac_cmd { parts.push("ctrl"); }
    if modifiers.alt { parts.push("alt"); }
    if modifiers.shift { parts.push("shift"); }
    let key_name = match key {
        egui::Key::F1 => "f1", egui::Key::F2 => "f2", /* ... F3-F12 ... */
        egui::Key::Num0 => "0", egui::Key::Num1 => "1", /* ... 2-9 ... */
        egui::Key::A => "a", egui::Key::B => "b", /* ... c-z ... */
        _ => return String::new(),
    };
    parts.push(key_name);
    parts.join("+")
}
```

### Step 4: Commit

```bash
git add crates/mudcore/src/script.rs crates/mudgui/src/session.rs crates/mudgui/src/app/mod.rs
git commit -m "feat: add mud.bind_key/unbind_key for custom keyboard shortcuts"
```

---

## Task 7: Message Routing

**Files:**
- Modify: `crates/mudgui/src/session.rs` (add route rules, apply in handle_text_with_widths)
- Modify: `crates/mudcore/src/script.rs` (add mud.add_route/remove_route API)

### Step 1: Add route rules to Session

```rust
/// Message routing rule
pub struct RouteRule {
    pub name: String,
    pub pattern: String,
    pub window: String,
    pub gag: bool,  // hide from main window
    compiled_regex: Option<regex::Regex>,
}

// In Session struct:
pub route_rules: Vec<RouteRule>,
```

### Step 2: Add mud.add_route/remove_route Lua API

```rust
// mud.add_route({ name, pattern, window, gag })
// mud.remove_route(name)
```

Add to MudContext:
```rust
pub route_additions: Vec<(String, String, String, bool)>,  // (name, pattern, window, gag)
pub route_removals: Vec<String>,
```

### Step 3: Apply routing in handle_text_with_widths

In the trigger processing section (around line 1500), after trigger evaluation:

```rust
// Check route rules
let stripped = strip_ansi(&text);
for rule in &self.route_rules {
    if let Some(ref re) = rule.compiled_regex {
        if re.is_match(&stripped) {
            // Route to window
            self.window_manager.route_message_with_widths(
                &rule.window,
                WindowMessage::new(text.clone()).with_widths(byte_widths.clone()),
            );
            if rule.gag {
                gagged = true;
            }
        }
    }
}
```

### Step 4: Commit

```bash
git add crates/mudcore/src/script.rs crates/mudgui/src/session.rs
git commit -m "feat: add message routing with mud.add_route/remove_route"
```

---

## Task 8: Script Debug Panel

**Files:**
- Modify: `crates/mudgui/src/app/sidebar.rs` (add Debug tab)
- Modify: `crates/mudgui/src/app/mod.rs` (add SidePanelTab::Debug variant)

### Step 1: Add Debug tab to SidePanelTab enum

```rust
enum SidePanelTab {
    Tools,
    Guide,
    Notes,
    Map,
    Terminal,
    Debug,  // NEW
}
```

### Step 2: Add render_debug_tab to sidebar.rs

```rust
fn render_debug_tab(&mut self, ui: &mut egui::Ui) {
    let Some(session) = self.session_manager.active_session_mut() else {
        ui.label("No active session");
        return;
    };

    // === Lua Console ===
    ui.heading("Lua Console");
    // Input field + execute button
    // Show last result

    // === Variables ===
    ui.heading("Variables");
    // List session.script_engine persistent_vars

    // === State Machines ===
    ui.heading("State Machines");
    // For each machine: name, current state, available transitions

    // === Event Log ===
    ui.heading("Recent Events");
    // Show last N events from event_bus.event_log
    // Format: timestamp | event_name | data
}
```

### Step 3: Commit

```bash
git add crates/mudgui/src/app/sidebar.rs crates/mudgui/src/app/mod.rs
git commit -m "feat: add script debug panel with Lua console, vars, state machines, event log"
```

---

## Task 9: Integration Testing & Documentation

**Files:**
- Modify: `docs/API.md` (document new APIs)
- Create: `scripts/examples/event_demo.lua` (example usage)

### Step 1: Create example script

```lua
-- scripts/examples/event_demo.lua
-- Demonstrates event system, trigger groups, and state machine

-- Register event handlers
mud.on("room_changed", "mud.echo('[Event] Room changed: ' .. (event_data or 'unknown'))")
mud.on("combat_start", "mud.echo('[Event] Combat started!')")

-- Set up trigger groups
mud.add_trigger({
    name = "detect_combat",
    pattern = "starts to fight",
    group = "detection",
    action = "mud.emit('combat_start')"
})

-- State machine
mud.state_machine("grinder", {
    initial = "idle",
    states = {
        idle = { enter = "mud.echo('[SM] Idle mode')" },
        fighting = {
            enter = "mud.enable_group('combat_skills', true)",
            exit = "mud.enable_group('combat_skills', false)",
            timeout = { seconds = 120, goto = "idle" },
        },
    },
    transitions = {
        { from = "idle", event = "combat_start", to = "fighting" },
        { from = "fighting", event = "combat_end", to = "idle" },
    },
})

-- Key bindings
mud.bind_key("f5", "mud.sm_reset('grinder'); mud.echo('Bot reset!')")
mud.bind_key("f6", "mud.echo('State: ' .. (mud.sm_current('grinder') or 'none'))")
```

### Step 2: Update API.md with new functions

Document: `mud.on`, `mud.off`, `mud.emit`, `mud.once`, `mud.enable_group`, `mud.state_machine`, `mud.sm_current`, `mud.sm_transition`, `mud.sm_reset`, `mud.bind_key`, `mud.unbind_key`, `mud.add_route`, `mud.remove_route`.

### Step 3: Commit

```bash
git add docs/API.md scripts/examples/event_demo.lua
git commit -m "docs: add event system API documentation and example script"
```

---

## Implementation Order & Dependencies

```
Task 1: EventBus Core          (independent)
Task 2: Lua Event API          (depends on Task 1)
Task 3: Wire EventBus          (depends on Task 2)
Task 4: Trigger Groups         (independent, can parallel with 1-3)
Task 5: State Machine          (depends on Task 3 for event integration)
Task 6: Key Bindings           (independent)
Task 7: Message Routing        (independent)
Task 8: Debug Panel            (depends on Tasks 3, 5 for data to display)
Task 9: Integration & Docs     (depends on all above)
```

**Parallelizable pairs:** Tasks 1+4, Tasks 6+7
