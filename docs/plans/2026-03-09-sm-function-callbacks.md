# SM Function Callback 支援 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 讓 State Machine 的 enter/exit callback 支援 Lua function（RegistryKey），與 string code 雙模式共存，使 ItemFarmEngage 等腳本中嵌套的 `collect_response`/`timer`/`on` 能使用 function 語法。

**Architecture:** 沿用 EventBus 已驗證的 PersistentFunction 借用模式。SM State 的 enter/exit 從 `Option<String>` 改為 `Option<StateCallback>`（`Code(String)` | `Function(RegistryKey)`）。TransitionResult 不持有 RegistryKey，而是回傳 `TransitionCallback` marker，caller 在 session.rs 中透過 SM 借用 key 執行。MudContext 的 SM 傳輸結構從 JSON 四元組改為 `SmDef` struct（攜帶 `LuaCallback`）。

**Tech Stack:** Rust (mlua RegistryKey), Lua 5.4, serde_json (僅用於 transitions 序列化)

**已驗證的先例模式：**
- `EventBus.emit()` → `EmittedCallback::PersistentFunction` → `get_handler_callback()` 借用
- `collect_response` function 模式：Lua table 存 function → `collect_mud_context` 取出建 RegistryKey
- NLL 允許同時借用 `self.event_bus`（不可變）和 `self.script_engine`（可變）

---

## Task 1: StateCallback enum + State 結構重構 (state_machine.rs)

**Files:**
- Modify: `crates/mudcore/src/state_machine.rs:1-16`

**Step 1: 在 state_machine.rs 頂部加入 imports 和 StateCallback**

在 `use` 區塊後、`State` 結構前加入：

```rust
use mlua::RegistryKey;
use crate::script::LuaCallback;
```

**Step 2: State 結構改用 LuaCallback**

將 State 從：
```rust
#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub enter_code: Option<String>,
    pub exit_code: Option<String>,
    pub timeout_secs: Option<f64>,
    pub timeout_goto: Option<String>,
}
```

改為：
```rust
pub struct State {
    pub name: String,
    pub enter: Option<LuaCallback>,
    pub exit: Option<LuaCallback>,
    pub timeout_secs: Option<f64>,
    pub timeout_goto: Option<String>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("name", &self.name)
            .field("enter", &self.enter)
            .field("exit", &self.exit)
            .field("timeout_secs", &self.timeout_secs)
            .field("timeout_goto", &self.timeout_goto)
            .finish()
    }
}
```

注意：移除 `Clone` derive（RegistryKey 不支援 Clone）。`LuaCallback` 已有 Debug impl。

**Step 3: 驗證編譯**

Run: `cargo check -p mudcore 2>&1 | head -50`
Expected: 會因 `do_transition` 中 `.clone()` 和測試中 `enter_code`/`exit_code` 欄位名而報錯。這是預期的，Task 2 會修復。

---

## Task 2: TransitionResult 改用 TransitionCallback + do_transition 重寫 (state_machine.rs)

**Files:**
- Modify: `crates/mudcore/src/state_machine.rs:26-88`

**Step 1: 定義 TransitionCallback enum**

將 TransitionResult 從：
```rust
#[derive(Debug)]
pub struct TransitionResult {
    pub exit_code: Option<String>,
    pub enter_code: Option<String>,
    pub old_state: String,
    pub new_state: String,
}
```

改為：
```rust
/// Transition callback：不持有 RegistryKey，caller 需透過 SM 借用
#[derive(Debug)]
pub enum TransitionCallback {
    /// 字串程式碼（從 State clone）
    Code(String),
    /// Function handler，caller 需透過 get_state_callback() 借用 RegistryKey
    PersistentFunction,
}

#[derive(Debug)]
pub struct TransitionResult {
    pub exit_callback: Option<TransitionCallback>,
    pub enter_callback: Option<TransitionCallback>,
    pub old_state: String,
    pub new_state: String,
}
```

**Step 2: 重寫 do_transition**

```rust
fn do_transition(&mut self, to: String) -> Option<TransitionResult> {
    if !self.states.contains_key(&to) { return None; }
    let old = self.current.clone();

    let exit_callback = self.states.get(&old).and_then(|s| {
        s.exit.as_ref().map(|cb| match cb {
            LuaCallback::Code(code) => TransitionCallback::Code(code.clone()),
            LuaCallback::Function(_) => TransitionCallback::PersistentFunction,
        })
    });

    let enter_callback = self.states.get(&to).and_then(|s| {
        s.enter.as_ref().map(|cb| match cb {
            LuaCallback::Code(code) => TransitionCallback::Code(code.clone()),
            LuaCallback::Function(_) => TransitionCallback::PersistentFunction,
        })
    });

    self.current = to.clone();
    self.timeout_at = self.states.get(&to)
        .and_then(|s| s.timeout_secs)
        .map(|secs| Instant::now() + Duration::from_secs_f64(secs));

    Some(TransitionResult { exit_callback, enter_callback, old_state: old, new_state: to })
}
```

**Step 3: 加入 get_state_callback 方法**

在 `StateMachine` impl 中加入（`current_state` 方法之後）：

```rust
/// 取得指定 state 的 enter/exit callback reference（用於 PersistentFunction）
pub fn get_state_callback(&self, state_name: &str, is_enter: bool) -> Option<&LuaCallback> {
    let state = self.states.get(state_name)?;
    if is_enter { state.enter.as_ref() } else { state.exit.as_ref() }
}
```

**Step 4: 驗證編譯**

Run: `cargo check -p mudcore 2>&1 | head -50`
Expected: 測試會報錯（`enter_code`/`exit_code` 欄位名），Task 3 修復。

---

## Task 3: 更新 state_machine.rs 測試

**Files:**
- Modify: `crates/mudcore/src/state_machine.rs:143-222`

**Step 1: 更新 make_test_machine**

```rust
fn make_test_machine() -> StateMachine {
    let mut states = HashMap::new();
    states.insert("idle".into(), State {
        name: "idle".into(),
        enter: Some(LuaCallback::Code("mud.echo('idle')".into())),
        exit: Some(LuaCallback::Code("mud.echo('leaving idle')".into())),
        timeout_secs: None, timeout_goto: None,
    });
    states.insert("fighting".into(), State {
        name: "fighting".into(),
        enter: Some(LuaCallback::Code("mud.echo('combat!')".into())),
        exit: None, timeout_secs: Some(60.0), timeout_goto: Some("idle".into()),
    });
    states.insert("looting".into(), State {
        name: "looting".into(),
        enter: Some(LuaCallback::Code("mud.send('get all')".into())),
        exit: None, timeout_secs: Some(10.0), timeout_goto: Some("idle".into()),
    });
    StateMachine::new("bot".into(), "idle".into(), states, vec![
        Transition { from: "idle".into(), event: "combat_start".into(), to: "fighting".into() },
        Transition { from: "fighting".into(), event: "combat_end".into(), to: "looting".into() },
        Transition { from: "looting".into(), event: "loot_done".into(), to: "idle".into() },
    ])
}
```

**Step 2: 更新測試斷言（exit_code/enter_code → exit_callback/enter_callback）**

`test_basic_transition`:
```rust
#[test]
fn test_basic_transition() {
    let mut sm = make_test_machine();
    assert_eq!(sm.current_state(), "idle");
    let r = sm.handle_event("combat_start").unwrap();
    assert_eq!(r.old_state, "idle");
    assert_eq!(r.new_state, "fighting");
    assert!(matches!(r.exit_callback, Some(TransitionCallback::Code(ref s)) if s.contains("leaving idle")));
    assert!(matches!(r.enter_callback, Some(TransitionCallback::Code(ref s)) if s.contains("combat!")));
}
```

**Step 3: 驗證所有 state_machine 測試通過**

Run: `cargo test -p mudcore -- state_machine 2>&1`
Expected: All tests pass.

---

## Task 4: MudContext SM 傳輸結構重構 (script.rs)

**Files:**
- Modify: `crates/mudcore/src/script.rs:93-94`

**Step 1: 定義 SmStateDef 和 SmDef 結構**

在 `MudContext` 之前或之內加入新結構，然後替換 `state_machine_defs` 欄位：

```rust
/// SM state 定義（用於兩階段傳輸）
pub struct SmStateDef {
    pub name: String,
    pub enter: Option<LuaCallback>,
    pub exit: Option<LuaCallback>,
    pub timeout_secs: Option<f64>,
    pub timeout_goto: Option<String>,
}

/// SM 定義（用於兩階段傳輸）
pub struct SmDef {
    pub name: String,
    pub initial: String,
    pub states: Vec<SmStateDef>,
    pub transitions: Vec<(String, String, String)>,  // (from, event, to)
}
```

MudContext 中：
```rust
// 替換：pub state_machine_defs: Vec<(String, String, String, String)>,
pub state_machine_defs: Vec<SmDef>,
```

**Step 2: 驗證編譯**

Run: `cargo check -p mudcore 2>&1 | head -30`
Expected: `mud.state_machine` API 和 `collect_mud_context` 會報錯（型別不匹配），Task 5 修復。

---

## Task 5: mud.state_machine Lua API 重寫 (script.rs)

**Files:**
- Modify: `crates/mudcore/src/script.rs:758-810` (mud.state_machine 函數)
- Modify: `crates/mudcore/src/script.rs:1383-1395` (collect_mud_context SM defs 段)

**Step 1: 重寫 mud.state_machine 函數**

關鍵改動：不再序列化到 JSON，而是將 enter/exit 作為 string 或 function 存入 Lua table，在 `collect_mud_context` 階段建 RegistryKey。

```rust
// mud.state_machine(name, definition)
let sm_fn = scope.create_function_mut(|lua, (name, def): (String, mlua::Table)| {
    let mud: mlua::Table = lua.globals().get("mud")?;
    let defs: mlua::Table = mud.get("_sm_defs")?;

    let initial: String = def.get("initial")?;
    let states_table: mlua::Table = def.get("states")?;

    // Store states as Lua table (preserving function references)
    let states_out = lua.create_table()?;
    for pair in states_table.pairs::<String, mlua::Table>() {
        let (state_name, state_def) = pair?;
        let state_entry = lua.create_table()?;
        state_entry.set("name", state_name.clone())?;

        // enter: preserve as-is (string or function)
        if let Ok(enter) = state_def.get::<mlua::Value>("enter") {
            match &enter {
                mlua::Value::String(_) | mlua::Value::Function(_) => {
                    state_entry.set("enter", enter)?;
                }
                _ => {}
            }
        }
        // exit: preserve as-is
        if let Ok(exit) = state_def.get::<mlua::Value>("exit") {
            match &exit {
                mlua::Value::String(_) | mlua::Value::Function(_) => {
                    state_entry.set("exit", exit)?;
                }
                _ => {}
            }
        }
        // timeout
        if let Ok(timeout) = state_def.get::<mlua::Table>("timeout") {
            if let (Ok(secs), Ok(goto)) = (
                timeout.get::<f64>("seconds"),
                timeout.get::<String>("target").or_else(|_| timeout.get::<String>("goto"))
            ) {
                state_entry.set("timeout_secs", secs)?;
                state_entry.set("timeout_goto", goto)?;
            }
        } else if let (Ok(secs), Ok(goto)) = (
            state_def.get::<f64>("timeout_secs"),
            state_def.get::<String>("timeout_goto")
        ) {
            state_entry.set("timeout_secs", secs)?;
            state_entry.set("timeout_goto", goto)?;
        }

        states_out.set(state_name, state_entry)?;
    }

    // Serialize transitions to JSON (simple string data, no functions)
    let trans_table: mlua::Table = def.get("transitions")?;
    let mut trans_arr = Vec::new();
    for i in 1..=trans_table.len()? {
        let t: mlua::Table = trans_table.get(i)?;
        trans_arr.push(serde_json::json!({
            "from": t.get::<String>("from")?,
            "event": t.get::<String>("event")?,
            "to": t.get::<String>("to")?,
        }));
    }

    let len = defs.len()? + 1;
    let entry = lua.create_table()?;
    entry.set(1, name.clone())?;
    entry.set(2, initial)?;
    entry.set(3, states_out)?;  // Lua table (not JSON string)
    entry.set(4, serde_json::Value::Array(trans_arr).to_string())?;  // transitions still JSON
    defs.set(len, entry)?;
    Ok(name)
})?;
mud.set("state_machine", sm_fn)?;
```

**Step 2: 重寫 collect_mud_context 的 SM defs 收集段**

```rust
// 收集 state machine definitions
if let Ok(defs) = mud.get::<mlua::Table>("_sm_defs") {
    for pair in defs.pairs::<i64, mlua::Table>() {
        if let Ok((_, entry)) = pair {
            let name: String = match entry.get(1) { Ok(v) => v, _ => continue };
            let initial: String = match entry.get(2) { Ok(v) => v, _ => continue };
            let states_table: mlua::Table = match entry.get(3) { Ok(v) => v, _ => continue };
            let trans_json: String = match entry.get(4) { Ok(v) => v, _ => continue };

            // Parse states from Lua table
            let mut state_defs = Vec::new();
            for state_pair in states_table.pairs::<String, mlua::Table>() {
                if let Ok((sname, sdef)) = state_pair {
                    let enter = Self::extract_lua_callback(&self.lua, &sdef, "enter");
                    let exit = Self::extract_lua_callback(&self.lua, &sdef, "exit");
                    let timeout_secs = sdef.get::<f64>("timeout_secs").ok();
                    let timeout_goto = sdef.get::<String>("timeout_goto").ok();

                    state_defs.push(SmStateDef {
                        name: sname,
                        enter,
                        exit,
                        timeout_secs,
                        timeout_goto,
                    });
                }
            }

            // Parse transitions from JSON
            let mut transitions = Vec::new();
            if let Ok(trans_val) = serde_json::from_str::<serde_json::Value>(&trans_json) {
                if let Some(arr) = trans_val.as_array() {
                    for t in arr {
                        if let (Some(from), Some(event), Some(to)) = (
                            t.get("from").and_then(|v| v.as_str()),
                            t.get("event").and_then(|v| v.as_str()),
                            t.get("to").and_then(|v| v.as_str()),
                        ) {
                            transitions.push((from.to_string(), event.to_string(), to.to_string()));
                        }
                    }
                }
            }

            context.state_machine_defs.push(SmDef {
                name,
                initial,
                states: state_defs,
                transitions,
            });
        }
    }
}
```

**Step 3: 加入 helper 方法 extract_lua_callback**

在 `ScriptEngine` impl 中加入：

```rust
/// 從 Lua table 中取出 string 或 function 值，轉換為 LuaCallback
fn extract_lua_callback(lua: &mlua::Lua, table: &mlua::Table, key: &str) -> Option<LuaCallback> {
    match table.get::<mlua::Value>(key) {
        Ok(mlua::Value::String(s)) => {
            let code = s.to_string_lossy();
            if code.is_empty() { None } else { Some(LuaCallback::Code(code)) }
        }
        Ok(mlua::Value::Function(_)) => {
            match table.get::<mlua::Value>(key) {
                Ok(val) => match lua.create_registry_value(val) {
                    Ok(key) => Some(LuaCallback::Function(key)),
                    Err(e) => {
                        tracing::error!("Failed to register SM callback function: {}", e);
                        None
                    }
                },
                _ => None,
            }
        }
        _ => None,
    }
}
```

**Step 4: 驗證 mudcore 編譯**

Run: `cargo check -p mudcore 2>&1 | head -30`
Expected: mudcore 編譯通過，mudgui 可能因 session.rs 中的舊結構而報錯。

---

## Task 6: session.rs apply_script_context SM 段重寫

**Files:**
- Modify: `crates/mudgui/src/session.rs:1396-1433`

**Step 1: 重寫 SM 定義處理段**

```rust
// 12. State machine definitions (before emissions so emit() in same scope can trigger SM)
for sm_def in context.state_machine_defs {
    let mut states = std::collections::HashMap::new();
    for sdef in sm_def.states {
        states.insert(sdef.name.clone(), mudcore::state_machine::State {
            name: sdef.name,
            enter: sdef.enter,
            exit: sdef.exit,
            timeout_secs: sdef.timeout_secs,
            timeout_goto: sdef.timeout_goto,
        });
    }
    let transitions: Vec<mudcore::state_machine::Transition> = sm_def.transitions
        .into_iter()
        .map(|(from, event, to)| mudcore::state_machine::Transition { from, event, to })
        .collect();

    let sm = mudcore::StateMachine::new(sm_def.name.clone(), sm_def.initial, states, transitions);
    tracing::info!("State machine '{}' created, initial state: '{}'", sm_def.name, sm.current_state());
    self.state_machines.add(sm);
    self.mark_sm_dirty();
}
```

**Step 2: 驗證編譯**

Run: `cargo check -p mudgui 2>&1 | head -30`
Expected: `execute_transition_callbacks` 會報錯（`exit_code`/`enter_code` 不存在），Task 7 修復。

---

## Task 7: execute_transition_callbacks 支援雙模式

**Files:**
- Modify: `crates/mudgui/src/session.rs:1570-1605`

**Step 1: 重寫 execute_transition_callbacks**

沿用 EventBus 的 PersistentFunction 借用模式：

```rust
/// Execute state machine transition callbacks (exit then enter)
fn execute_transition_callbacks(&mut self, machine_name: &str, result: mudcore::state_machine::TransitionResult) {
    use mudcore::state_machine::TransitionCallback;
    use mudcore::script::LuaCallback;

    tracing::info!("[SM:{}] {} -> {}", machine_name, result.old_state, result.new_state);
    self.mark_sm_dirty();

    // Execute exit callback
    if let Some(cb) = result.exit_callback {
        self.sync_sm_to_lua();
        match cb {
            TransitionCallback::Code(code) => {
                match self.script_engine.execute_inline(&code, "", &[], false) {
                    Ok(ctx) => self.apply_script_context(ctx),
                    Err(e) => {
                        tracing::error!("[SM:{}] exit callback error: {}", machine_name, e);
                        self.system_message(&format!("[SM:{}] Exit Callback Error: {}", machine_name, e));
                    }
                }
            }
            TransitionCallback::PersistentFunction => {
                let exec_result = {
                    if let Some(sm) = self.state_machines.get(machine_name) {
                        if let Some(LuaCallback::Function(key)) = sm.get_state_callback(&result.old_state, false) {
                            Some(self.script_engine.execute_lua_callback_ref(key, "SM_EXIT"))
                        } else { None }
                    } else { None }
                };
                match exec_result {
                    Some(Ok(ctx)) => self.apply_script_context(ctx),
                    Some(Err(e)) => {
                        tracing::error!("[SM:{}] exit callback function error: {}", machine_name, e);
                        self.system_message(&format!("[SM:{}] Exit Callback Error: {}", machine_name, e));
                    }
                    None => {}
                }
            }
        }
    }

    // Execute enter callback
    if let Some(cb) = result.enter_callback {
        self.sync_sm_to_lua();
        match cb {
            TransitionCallback::Code(code) => {
                match self.script_engine.execute_inline(&code, "", &[], false) {
                    Ok(ctx) => self.apply_script_context(ctx),
                    Err(e) => {
                        tracing::error!("[SM:{}] enter callback error: {}", machine_name, e);
                        self.system_message(&format!("[SM:{}] Enter Callback Error: {}", machine_name, e));
                    }
                }
            }
            TransitionCallback::PersistentFunction => {
                let exec_result = {
                    if let Some(sm) = self.state_machines.get(machine_name) {
                        if let Some(LuaCallback::Function(key)) = sm.get_state_callback(&result.new_state, true) {
                            Some(self.script_engine.execute_lua_callback_ref(key, "SM_ENTER"))
                        } else { None }
                    } else { None }
                };
                match exec_result {
                    Some(Ok(ctx)) => self.apply_script_context(ctx),
                    Some(Err(e)) => {
                        tracing::error!("[SM:{}] enter callback function error: {}", machine_name, e);
                        self.system_message(&format!("[SM:{}] Enter Callback Error: {}", machine_name, e));
                    }
                    None => {}
                }
            }
        }
    }

    // Emit state_changed event
    let data = format!(r#"{{"machine":"{}","old":"{}","new":"{}"}}"#,
        machine_name, result.old_state, result.new_state);
    self.emit_event("state_changed", Some(data));
}
```

**Step 2: 全專案編譯**

Run: `cargo check 2>&1 | head -50`
Expected: 全部通過。

---

## Task 8: 全面測試

**Step 1: 跑 mudcore 單元測試**

Run: `cargo test -p mudcore 2>&1`
Expected: 所有測試通過，包括 state_machine 和 script tests。

**Step 2: 跑完整測試**

Run: `cargo test 2>&1`
Expected: 所有測試通過。

**Step 3: 完整 build**

Run: `cargo build 2>&1`
Expected: Build 成功。

**Step 4: Commit**

```bash
git add crates/mudcore/src/state_machine.rs crates/mudcore/src/script.rs crates/mudgui/src/session.rs
git commit -m "feat(sm): support function callbacks in state machine enter/exit

State machine enter/exit callbacks now accept both string code and Lua
function references via LuaCallback enum. Uses PersistentFunction
borrowing pattern (same as EventBus) to handle RegistryKey lifecycle.

- State.enter_code/exit_code -> State.enter/exit: Option<LuaCallback>
- TransitionResult uses TransitionCallback enum with PersistentFunction
- MudContext.state_machine_defs uses SmDef struct instead of JSON tuple
- mud.state_machine() preserves function refs through Lua table
- session.rs borrows RegistryKey from SM for execution (NLL safe)"
```

---

## Task 9: LuaLS 型別定義更新

**Files:**
- Modify: `scripts/types/mud.lua`

**Step 1: 更新 mud.state_machine 的 enter/exit 型別**

找到 `mud.state_machine` 的型別定義，將 `enter` 和 `exit` 欄位從 `string?` 改為 `string|fun()?`。

---

## Task 10: MCP Live Test

**Step 1: 重新編譯並啟動客戶端**

**Step 2: 透過 MCP evaluate_lua 測試 string 模式（向下相容）**

```lua
mud.state_machine("test_str", {
    initial = "idle",
    states = {
        idle = { enter = "mud.echo('[test_str] entered idle')" },
        active = { enter = "mud.echo('[test_str] entered active')" },
    },
    transitions = {
        { from = "idle", event = "go", to = "active" },
        { from = "active", event = "stop", to = "idle" },
    },
})
mud.sm_transition("test_str", "go")
```

Expected: 看到 `[test_str] entered active` 輸出。

**Step 3: 透過 MCP evaluate_lua 測試 function 模式**

```lua
mud.state_machine("test_fn", {
    initial = "idle",
    states = {
        idle = {
            enter = function()
                mud.echo("[test_fn] idle enter (function mode)")
            end,
        },
        active = {
            enter = function()
                mud.echo("[test_fn] active enter (function mode)")
                mud.timer(1.0, function()
                    mud.echo("[test_fn] timer fired inside SM enter!")
                end)
            end,
            exit = function()
                mud.echo("[test_fn] active exit (function mode)")
            end,
        },
    },
    transitions = {
        { from = "idle", event = "go", to = "active" },
        { from = "active", event = "stop", to = "idle" },
    },
})
mud.sm_transition("test_fn", "go")
```

Expected: 看到 `[test_fn] active enter (function mode)`，1 秒後看到 `timer fired`。

**Step 4: 測試 function enter 中使用 collect_response**

```lua
mud.sm_transition("test_fn", "stop")
-- 確認 exit function 被呼叫
```

Expected: 看到 `[test_fn] active exit (function mode)` 和 `[test_fn] idle enter (function mode)`。

**Step 5: 清理測試 SM**

```lua
mud.sm_remove("test_str")
mud.sm_remove("test_fn")
```

---

## Task 11: ItemFarmEngage.lua 轉換（部分）

**Files:**
- Modify: `scripts/modules/ItemFarmEngage.lua`

**背景：** ItemFarmEngage 的 SM enter 字串使用了 `string.format` + `[==[]==]` 嵌套，包含 `collect_response`、`timer`、`on` 等需要 function 的 API。有了 function enter 支援後，可以逐步將這些嵌套字串改為 function enter + 閉包。

**注意：** 此 Task 工作量大（14+ 處），應逐個 state 轉換並測試。每個 state 的 enter 從 `string.format([[...]])` 改為 `function() ... end`，內部的 `collect_response`/`timer` 改用 function callback。

**Step 1: 先轉換 verify_mob state 作為範例**

原始（string.format + 嵌套字串）：
```lua
states.verify_mob = {
    enter = string.format([[
        _G.ItemFarm.engage.echo("Confirming target in room...")
        mud.collect_response("l", [==[
            local td = %s
            for _, line in ipairs(_G._collected_lines or {}) do ...
            end
        ]==])
    ]], td_ser),
}
```

改為（function + 閉包）：
```lua
local td = job.engage.target_display  -- 在定義時捕獲
states.verify_mob = {
    enter = function()
        _G.ItemFarm.engage.echo("Confirming target in room...")
        mud.collect_response("l", function(lines)
            for _, line in ipairs(lines or {}) do
                local matched = false
                if type(td) == "table" then
                    for _, kw in ipairs(td) do
                        if string.find(line, kw, 1, true) then matched = true; break end
                    end
                elseif type(td) == "string" then
                    matched = string.find(line, td, 1, true) ~= nil
                end
                if matched
                   and not string.find(line, "屍體", 1, true)
                   and not string.find(line, "corpse", 1, true)
                then
                    if mud.sm_current("itemfarm_engage") == "verify_mob" then
                        mud.sm_transition("itemfarm_engage", "mob_sighted")
                    end
                    return
                end
            end
            if mud.sm_current("itemfarm_engage") == "verify_mob" then
                mud.sm_transition("itemfarm_engage", "mob_not_here")
            end
        end)
    end,
    timeout_secs = 5.0,
    timeout_goto = "verify_loc",
}
```

**優勢：** 閉包直接捕獲 `td` 變數，不需要 `serialize_target` + `string.format` + `%s` 注入。程式碼更清晰、更安全。

**Step 2: 逐步轉換其他 states（verify_loc, dispelling, dispel_check, check_dispel 等）**

每個 state 使用相同模式：`string.format([[...]])` → `function() ... end`，閉包捕獲所需變數。

**Step 3: 測試 ItemFarm 流程**

完整跑一次 ItemFarm 流程，確認所有 SM state 轉換正常。

---

## 風險與注意事項

1. **Clone 移除連鎖**：State 移除 Clone 後，StateMachine 和 StateMachineManager 也不能 Clone。需確認 session.rs 中沒有 clone 這些物件的地方（已確認沒有）。

2. **RegistryKey 生命週期**：SM 被 `sm_remove` 時，State 被 drop，RegistryKey 自動釋放。SM 被重新定義（相同名稱）時，舊 SM 被替換（HashMap insert），舊 RegistryKey 也會被 drop。

3. **NLL 借用安全性**：`execute_transition_callbacks` 中同時借用 `self.state_machines`（不可變）和 `self.script_engine`（可變）。NLL 允許此操作因為它們是 struct 的不同欄位。已在 EventBus 中驗證此模式可行。

4. **向下相容**：string 模式完全保留。所有現有腳本無需修改即可繼續運作。
