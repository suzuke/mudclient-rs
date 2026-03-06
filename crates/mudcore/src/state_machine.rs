//! 狀態機框架
//!
//! 支援命名的狀態機實例，每個狀態有 enter/exit callback、timeout 自動轉換

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A single state definition
#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub enter_code: Option<String>,
    pub exit_code: Option<String>,
    pub timeout_secs: Option<f64>,
    pub timeout_goto: Option<String>,
}

/// A transition rule
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub event: String,
    pub to: String,
}

/// Result of a state transition (exit/enter callbacks to run)
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

    fn do_transition(&mut self, to: String) -> Option<TransitionResult> {
        if !self.states.contains_key(&to) { return None; }
        let old = self.current.clone();
        let exit_code = self.states.get(&old).and_then(|s| s.exit_code.clone());
        let enter_code = self.states.get(&to).and_then(|s| s.enter_code.clone());
        self.current = to.clone();
        self.timeout_at = self.states.get(&to)
            .and_then(|s| s.timeout_secs)
            .map(|secs| Instant::now() + Duration::from_secs_f64(secs));
        Some(TransitionResult { exit_code, enter_code, old_state: old, new_state: to })
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

    /// Check timeouts across ALL machines
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

impl Default for StateMachineManager {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_machine() -> StateMachine {
        let mut states = HashMap::new();
        states.insert("idle".into(), State {
            name: "idle".into(), enter_code: Some("mud.echo('idle')".into()),
            exit_code: Some("mud.echo('leaving idle')".into()),
            timeout_secs: None, timeout_goto: None,
        });
        states.insert("fighting".into(), State {
            name: "fighting".into(), enter_code: Some("mud.echo('combat!')".into()),
            exit_code: None, timeout_secs: Some(60.0), timeout_goto: Some("idle".into()),
        });
        states.insert("looting".into(), State {
            name: "looting".into(), enter_code: Some("mud.send('get all')".into()),
            exit_code: None, timeout_secs: Some(10.0), timeout_goto: Some("idle".into()),
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
        assert!(r.exit_code.unwrap().contains("leaving idle"));
        assert!(r.enter_code.unwrap().contains("combat!"));
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
