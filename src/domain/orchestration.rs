use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

// Layer 2 orchestration: a chain is a deterministic finite state machine over connector tools. Each state
// exposes only its `allowed_tools`, transitions follow `on_done`, and the machine owns the path - the model
// only proposes a tool within the current state. This complements, never replaces, the Layer 1 governance
// gate (`governance`): both apply on every call. Triggers/activation are a later layer and not modeled here.

// A deterministic precondition checked at the FSM seam before a state runs. `read_before_write` mirrors the
// Layer 1 floor so a skipped read structurally blocks the write state, independent of the per-call gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Guard {
    ReadBeforeWrite,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainState {
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_done: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<Guard>,
    #[serde(default)]
    pub terminal: bool,
}

impl ChainState {
    // The model may propose only tools this state lists. An empty list (e.g. a terminal answer) permits none.
    pub fn permits(&self, tool: &str) -> bool {
        self.allowed_tools.iter().any(|t| t == tool)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    pub initial: String,
    pub states: BTreeMap<String, ChainState>,
}

// The full orchestration object. `triggers` ride in the schema but are a later layer, so only `chains` is
// modeled today; an unknown trigger block parses and is ignored.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Orchestration {
    #[serde(default)]
    pub chains: BTreeMap<String, Chain>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChainError {
    #[error("initial state `{0}` is not defined")]
    UnknownInitial(String),
    #[error("state `{from}` transitions to undefined state `{to}`")]
    UnknownTransition { from: String, to: String },
    #[error("non-terminal state `{0}` has no on_done transition")]
    DanglingState(String),
    #[error("non-terminal state `{0}` exposes no allowed_tools, so it can never advance")]
    StateWithoutTools(String),
    #[error("terminal state `{0}` must not declare on_done")]
    TerminalWithTransition(String),
    #[error("no terminal reachable from `{0}` (cycle or dead end)")]
    NoTerminalReachable(String),
    #[error("state `{0}` is unreachable from the initial state")]
    UnreachableState(String),
}

impl Chain {
    pub fn state(&self, name: &str) -> Option<&ChainState> {
        self.states.get(name)
    }

    // Structural soundness before a run: the initial exists, every transition resolves, terminals carry no
    // on_done while non-terminals do, the on_done walk reaches a terminal without looping, and no state is
    // orphaned. v1 chains are linear (single on_done path), so reachability is that walk.
    pub fn validate(&self) -> Result<(), ChainError> {
        if !self.states.contains_key(&self.initial) {
            return Err(ChainError::UnknownInitial(self.initial.clone()));
        }
        for (name, st) in &self.states {
            if !st.terminal && st.allowed_tools.is_empty() {
                return Err(ChainError::StateWithoutTools(name.clone()));
            }
            match (&st.on_done, st.terminal) {
                (Some(to), false) => {
                    if !self.states.contains_key(to) {
                        return Err(ChainError::UnknownTransition {
                            from: name.clone(),
                            to: to.clone(),
                        });
                    }
                }
                (Some(_), true) => {
                    return Err(ChainError::TerminalWithTransition(
                        name.clone(),
                    ));
                }
                (None, false) => {
                    return Err(ChainError::DanglingState(name.clone()));
                }
                (None, true) => {}
            }
        }

        let mut seen = BTreeSet::new();
        let mut cur = self.initial.as_str();
        loop {
            if !seen.insert(cur.to_string()) {
                return Err(ChainError::NoTerminalReachable(
                    self.initial.clone(),
                ));
            }
            let st = self.states.get(cur).expect("transitions validated above");
            if st.terminal {
                break;
            }
            cur = st
                .on_done
                .as_deref()
                .expect("non-terminal validated to carry on_done");
        }
        for name in self.states.keys() {
            if !seen.contains(name) {
                return Err(ChainError::UnreachableState(name.clone()));
            }
        }
        Ok(())
    }
}

pub fn parse_chain(json: &str) -> serde_json::Result<Chain> {
    serde_json::from_str(json)
}

pub fn parse_orchestration(json: &str) -> serde_json::Result<Orchestration> {
    serde_json::from_str(json)
}
