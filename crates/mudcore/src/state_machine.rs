//! 狀態機框架
//!
//! 支援命名的狀態機實例，每個狀態有 enter/exit callback、timeout 自動轉換

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::script::LuaCallback;

/// A single state definition
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

/// A transition rule
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub event: String,
    pub to: String,
}

/// Describes which kind of callback to execute after a transition
#[derive(Debug)]
pub enum TransitionCallback {
    Code(String),
    PersistentFunction,
}

/// Result of a state transition (exit/enter callbacks to run)
#[derive(Debug)]
pub struct TransitionResult {
    pub exit_callback: Option<TransitionCallback>,
    pub enter_callback: Option<TransitionCallback>,
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
    pub timeout_at: Option<Instant>,
}

impl StateMachine {
    pub fn new(name: String, initial: String, states: HashMap<String, State>, transitions: Vec<Transition>) -> Self {
        let timeout_at = states.get(&initial)
            .and_then(|s| s.timeout_secs)
            .map(|secs| Instant::now() + Duration::from_secs_f64(secs));
        Self { name, current: initial.clone(), initial, states, transitions, timeout_at }
    }

    /// Try to transition on an event
    pub fn handle_event(&mut self, event: &str) -> Option<TransitionResult> {
        let to = self.transitions.iter()
            .find(|t| t.from == self.current && t.event == event)
            .map(|t| t.to.clone())?;
        self.do_transition(to)
    }

    /// Check if current state has timed out
    pub fn check_timeout(&mut self) -> Option<TransitionResult> {
        let timeout_at = self.timeout_at?;
        if Instant::now() < timeout_at { return None; }
        let goto = self.states.get(&self.current)?.timeout_goto.clone()?;
        self.do_transition(goto)
    }

    /// Reset to initial state
    pub fn reset(&mut self) -> Option<TransitionResult> {
        let initial = self.initial.clone();
        self.do_transition(initial)
    }

    pub fn current_state(&self) -> &str { &self.current }

    /// Get a reference to a state's enter or exit callback
    pub fn get_state_callback(&self, state_name: &str, is_enter: bool) -> Option<&LuaCallback> {
        let state = self.states.get(state_name)?;
        if is_enter { state.enter.as_ref() } else { state.exit.as_ref() }
    }

    fn do_transition(&mut self, to: String) -> Option<TransitionResult> {
        if !self.states.contains_key(&to) { return None; }
        let old = self.current.clone();
        let exit_callback = self.states.get(&old).and_then(|s| s.exit.as_ref()).map(|cb| match cb {
            LuaCallback::Code(s) => TransitionCallback::Code(s.clone()),
            LuaCallback::Function(_) => TransitionCallback::PersistentFunction,
        });
        let enter_callback = self.states.get(&to).and_then(|s| s.enter.as_ref()).map(|cb| match cb {
            LuaCallback::Code(s) => TransitionCallback::Code(s.clone()),
            LuaCallback::Function(_) => TransitionCallback::PersistentFunction,
        });
        self.current = to.clone();
        self.timeout_at = self.states.get(&to)
            .and_then(|s| s.timeout_secs)
            .map(|secs| Instant::now() + Duration::from_secs_f64(secs));
        Some(TransitionResult { exit_callback, enter_callback, old_state: old, new_state: to })
    }
}

/// Manages multiple named state machines
pub struct StateMachineManager {
    pub machines: HashMap<String, StateMachine>,
}

impl StateMachineManager {
    pub fn new() -> Self { Self { machines: HashMap::new() } }

    pub fn add(&mut self, machine: StateMachine) {
        self.machines.insert(machine.name.clone(), machine);
    }

    pub fn get_all_states(&self) -> HashMap<String, String> {
        self.machines.iter().map(|(k, v)| (k.clone(), v.current_state().to_string())).collect()
    }

    pub fn get(&self, name: &str) -> Option<&StateMachine> { self.machines.get(name) }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut StateMachine> { self.machines.get_mut(name) }
    pub fn remove(&mut self, name: &str) -> Option<StateMachine> { self.machines.remove(name) }

    /// Handle event across ALL machines
    pub fn handle_event(&mut self, event: &str) -> Vec<(String, TransitionResult)> {
        let mut results = Vec::new();
        for (name, machine) in &mut self.machines {
            if let Some(result) = machine.handle_event(event) {
                results.push((name.clone(), result));
            }
        }
        results
    }

    /// Check and return the first expired timeout (avoids Vec allocation)
    pub fn check_first_timeout(&mut self) -> Option<(String, TransitionResult)> {
        for (name, machine) in &mut self.machines {
            if let Some(result) = machine.check_timeout() {
                return Some((name.clone(), result));
            }
        }
        None
    }

    /// Return the earliest timeout_at across all machines (for scheduling UI repaint)
    pub fn next_timeout(&self) -> Option<Instant> {
        self.machines.values()
            .filter_map(|m| m.timeout_at)
            .min()
    }
}

impl Default for StateMachineManager {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_machine() -> StateMachine {
        let mut states = HashMap::new();
        states.insert("idle".into(), State {
            name: "idle".into(), enter: Some(LuaCallback::Code("mud.echo('idle')".into())),
            exit: Some(LuaCallback::Code("mud.echo('leaving idle')".into())),
            timeout_secs: None, timeout_goto: None,
        });
        states.insert("fighting".into(), State {
            name: "fighting".into(), enter: Some(LuaCallback::Code("mud.echo('combat!')".into())),
            exit: None, timeout_secs: Some(60.0), timeout_goto: Some("idle".into()),
        });
        states.insert("looting".into(), State {
            name: "looting".into(), enter: Some(LuaCallback::Code("mud.send('get all')".into())),
            exit: None, timeout_secs: Some(10.0), timeout_goto: Some("idle".into()),
        });
        StateMachine::new("bot".into(), "idle".into(), states, vec![
            Transition { from: "idle".into(), event: "combat_start".into(), to: "fighting".into() },
            Transition { from: "fighting".into(), event: "combat_end".into(), to: "looting".into() },
            Transition { from: "looting".into(), event: "loot_done".into(), to: "idle".into() },
        ])
    }

    #[test]
    fn test_basic_transition() {
        let mut sm = make_test_machine();
        assert_eq!(sm.current_state(), "idle");
        let r = sm.handle_event("combat_start").unwrap();
        assert_eq!(r.old_state, "idle");
        assert_eq!(r.new_state, "fighting");
        assert!(matches!(&r.exit_callback, Some(TransitionCallback::Code(s)) if s.contains("leaving idle")));
        assert!(matches!(&r.enter_callback, Some(TransitionCallback::Code(s)) if s.contains("combat!")));
    }

    #[test]
    fn test_no_matching_transition() {
        let mut sm = make_test_machine();
        assert!(sm.handle_event("combat_end").is_none());
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
        let r = sm.reset().unwrap();
        assert_eq!(r.new_state, "idle");
        assert_eq!(sm.current_state(), "idle");
    }

    #[test]
    fn test_timeout_set() {
        let mut sm = make_test_machine();
        assert!(sm.timeout_at.is_none());
        sm.handle_event("combat_start");
        assert!(sm.timeout_at.is_some());
    }

    #[test]
    fn test_manager_broadcast() {
        let mut mgr = StateMachineManager::new();
        mgr.add(make_test_machine());
        let results = mgr.handle_event("combat_start");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "bot");
    }
}
