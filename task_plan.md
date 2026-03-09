# Task: Refactor LuaCallback for all callback APIs

## Goal
Rename `TimerCallback` → `LuaCallback` and apply to all callback APIs that currently only accept string code.

## APIs to change
| API | Current | Target | Notes |
|-----|---------|--------|-------|
| `mud.timer` | ✅ Done (TimerCallback) | Rename to LuaCallback | |
| `mud.collect_response` | `(String, String)` | `(String, LuaCallback)` | Crosses thread via channel |
| `mud.send_chain` | uses collect_response | Same | |
| `mud.on` / `mud.once` | `(String, String, i32, bool)` | `(String, LuaCallback, i32, bool)` | EventBus stores handlers |
| `mud.ask_llm` | `LlmRequest { callback_code: String }` | `LlmRequest { callback: LuaCallback }` | Crosses thread |
| SM enter/exit | String in JSON | **Keep string-only** | Serialized to JSON, too complex |

## RegistryKey: Send + Sync ✅
Confirmed: `mlua::RegistryKey` is `Send + Sync`, safe to pass through channels.

## Phases

### Phase 1: Rename TimerCallback → LuaCallback [pending]
- `script.rs`: rename enum, update all references
- `session.rs`: update `ActiveTimer`, `check_timers`, `apply_script_context`
- Add `execute_lua_callback(&self, key: RegistryKey) -> Result<MudContext>` (rename from `execute_timer_function`)

### Phase 2: collect_response + send_chain [pending]
- `MudContext.response_collectors`: `Vec<(String, String)>` → `Vec<(String, LuaCallback)>`
- `Command::CollectResponse`: `callback_code: String` → `callback: LuaCallback`
- `NetMessage::CollectedResponse`: same
- `session.rs`: `execute_collected_response` handle both modes
- `app/mod.rs`: update channel handling
- `script.rs`: `mud.collect_response` accept Value, `mud.send_chain` accept Value

### Phase 3: Event system (on/once) [pending]
- `event.rs`: `EventHandler.lua_code: String` → `EventHandler.callback: LuaCallback`
- `EventBus::on()` signature change
- `EventBus::emit()` return type change
- `MudContext.event_registrations`: tuple type change
- `session.rs`: event handler execution handle both modes

### Phase 4: ask_llm [pending]
- `LlmRequest.callback_code` → `LlmRequest.callback: LuaCallback`
- LLM result handling: function mode receives result via `_G._llm_result`
- `session.rs`: LLM response handler

### Phase 5: Update types, tests, docs [pending]
- Update LuaLS type definitions
- Update tests
- Update docs
- List scripts that can use new function syntax

## Files to modify
- `crates/mudcore/src/script.rs` — LuaCallback enum, mud.* APIs, collection
- `crates/mudcore/src/event.rs` — EventHandler, EventBus
- `crates/mudgui/src/session.rs` — Command, ActiveTimer, apply_script_context, execute_*
- `crates/mudgui/src/app/mod.rs` — NetMessage, channel handling
- `scripts/types/mud.lua` — LuaLS definitions
- `docs/LuaAPI.md` — API documentation
