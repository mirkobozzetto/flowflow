// The Layer 2 chain FSM: parse, structural validation, transitions, and per-state tool scoping. Pure
// domain - no IO, no agent - so the deterministic state machine is locked independently of the runtime.

use flowflow::domain::orchestration::{
    parse_chain, parse_orchestration, Chain, ChainError, Guard,
};

const FIND_READ_ACT_ANSWER: &str = r#"{
  "initial": "find",
  "states": {
    "find":   { "allowed_tools": ["list"], "on_done": "read" },
    "read":   { "allowed_tools": ["get"],  "on_done": "act" },
    "act":    { "allowed_tools": ["write"], "guard": "read_before_write", "on_done": "answer" },
    "answer": { "terminal": true }
  }
}"#;

fn chain() -> Chain {
    parse_chain(FIND_READ_ACT_ANSWER).expect("parses")
}

#[test]
fn valid_chain_validates() {
    chain().validate().expect("sound chain");
}

#[test]
fn transitions_follow_on_done_in_order() {
    let c = chain();
    assert_eq!(c.initial, "find");
    assert_eq!(c.state("find").unwrap().on_done.as_deref(), Some("read"));
    assert_eq!(c.state("read").unwrap().on_done.as_deref(), Some("act"));
    assert_eq!(c.state("act").unwrap().on_done.as_deref(), Some("answer"));
    assert!(c.state("answer").unwrap().terminal);
    assert!(c.state("answer").unwrap().on_done.is_none());
}

#[test]
fn act_state_carries_read_before_write_guard() {
    assert_eq!(
        chain().state("act").unwrap().guard,
        Some(Guard::ReadBeforeWrite)
    );
    assert!(chain().state("find").unwrap().guard.is_none());
}

#[test]
fn permits_only_the_states_own_tools() {
    let c = chain();
    assert!(c.state("find").unwrap().permits("list"));
    assert!(!c.state("find").unwrap().permits("get"));
    assert!(!c.state("find").unwrap().permits("write"));
    // a terminal state with no allowed_tools permits nothing.
    assert!(!c.state("answer").unwrap().permits("list"));
}

#[test]
fn unknown_initial_rejected() {
    let c = parse_chain(
        r#"{ "initial": "ghost", "states": { "a": { "terminal": true } } }"#,
    )
    .unwrap();
    assert_eq!(
        c.validate(),
        Err(ChainError::UnknownInitial("ghost".into()))
    );
}

#[test]
fn transition_to_undefined_state_rejected() {
    let c = parse_chain(
        r#"{ "initial": "a", "states": { "a": { "allowed_tools": ["x"], "on_done": "nowhere" } } }"#,
    )
    .unwrap();
    assert_eq!(
        c.validate(),
        Err(ChainError::UnknownTransition {
            from: "a".into(),
            to: "nowhere".into()
        })
    );
}

#[test]
fn non_terminal_without_transition_rejected() {
    let c = parse_chain(
        r#"{ "initial": "a", "states": { "a": { "allowed_tools": ["x"] } } }"#,
    )
    .unwrap();
    assert_eq!(c.validate(), Err(ChainError::DanglingState("a".into())));
}

#[test]
fn non_terminal_without_tools_rejected() {
    let c = parse_chain(
        r#"{ "initial": "a", "states": { "a": { "on_done": "b" }, "b": { "terminal": true } } }"#,
    )
    .unwrap();
    assert_eq!(c.validate(), Err(ChainError::StateWithoutTools("a".into())));
}

#[test]
fn unknown_state_field_rejected() {
    // a mistyped field (here `gaurd`) must fail parsing rather than silently default, so a typo cannot
    // disable the FSM-seam guard.
    assert!(parse_chain(
        r#"{ "initial": "a", "states": { "a": { "allowed_tools": ["x"], "gaurd": "read_before_write", "on_done": "b" }, "b": { "terminal": true } } }"#
    )
    .is_err());
}

#[test]
fn terminal_with_transition_rejected() {
    let c = parse_chain(
        r#"{ "initial": "a", "states": { "a": { "terminal": true, "on_done": "b" }, "b": { "terminal": true } } }"#,
    )
    .unwrap();
    assert_eq!(
        c.validate(),
        Err(ChainError::TerminalWithTransition("a".into()))
    );
}

#[test]
fn cycle_without_terminal_rejected() {
    let c = parse_chain(
        r#"{ "initial": "a", "states": { "a": { "allowed_tools": ["x"], "on_done": "b" }, "b": { "allowed_tools": ["x"], "on_done": "a" } } }"#,
    )
    .unwrap();
    assert_eq!(
        c.validate(),
        Err(ChainError::NoTerminalReachable("a".into()))
    );
}

#[test]
fn orphan_state_rejected() {
    let c = parse_chain(
        r#"{ "initial": "a", "states": { "a": { "terminal": true }, "stray": { "terminal": true } } }"#,
    )
    .unwrap();
    assert_eq!(
        c.validate(),
        Err(ChainError::UnreachableState("stray".into()))
    );
}

#[test]
fn orchestration_parses_named_chains_and_ignores_triggers() {
    let orch = parse_orchestration(
        r#"{
          "triggers": [ { "kind": "keyword", "words": ["sync"], "then": "sync_chain" } ],
          "chains": { "sync_chain": {
            "initial": "find",
            "states": { "find": { "allowed_tools": ["list"], "on_done": "answer" }, "answer": { "terminal": true } }
          } }
        }"#,
    )
    .expect("trigger block is tolerated, chains parse");
    let c = orch.chains.get("sync_chain").expect("named chain");
    c.validate().expect("sound");
    assert_eq!(c.initial, "find");
}
