use flowflow::application::rag::{
    estimate_tokens, RagTrace, StepOutcome, TraceStep,
};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn step(name: &str, outcome: StepOutcome) -> TraceStep {
    TraceStep {
        step: name.to_string(),
        model: Some("gpt-5.4-mini".to_string()),
        tokens_in: Some(120),
        tokens_out: Some(40),
        approx: true,
        latency_ms: 250,
        retries: 0,
        outcome,
    }
}

#[test]
fn estimate_tokens_scales_with_length() {
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("abcd"), 1);
    assert_eq!(estimate_tokens(&"x".repeat(400)), 100);
}

#[test]
fn trace_json_round_trips() {
    let mut trace = RagTrace::default();
    trace.push(step("rewrite", StepOutcome::Skipped));
    trace.push(step("judge", StepOutcome::Ok));
    trace.push(step("temporal", StepOutcome::Fallback));
    trace.total_ms = 4200;

    let json = trace.to_json();
    let back = RagTrace::from_json(&json).expect("parse back");
    assert_eq!(back, trace);
    assert_eq!(back.steps.len(), 3);
    assert_eq!(back.steps[0].outcome, StepOutcome::Skipped);
    assert!(
        json.contains("\"skipped\""),
        "outcome serializes snake_case"
    );
}

#[test]
fn from_json_rejects_garbage() {
    assert!(RagTrace::from_json("not json").is_none());
    assert!(RagTrace::from_json("{\"steps\": 3}").is_none());
}

#[test]
fn summary_reports_count_and_seconds() {
    let mut trace = RagTrace::default();
    trace.push(step("embed", StepOutcome::Ok));
    trace.push(step("agent", StepOutcome::Ok));
    trace.total_ms = 6234;
    let (n, secs) = trace.summary();
    assert_eq!(n, 2);
    assert!((secs - 6.234).abs() < 1e-9);
}

#[test]
fn message_trace_round_trips_in_db() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("trace test").unwrap();
    let msg = db.add_message(&conv.id, "bot", "answer", None).unwrap();

    assert_eq!(db.message_trace(&msg.id), None, "unset trace reads None");

    let mut trace = RagTrace::default();
    trace.push(step("search", StepOutcome::Ok));
    trace.total_ms = 900;
    db.set_message_trace(&msg.id, &trace.to_json()).unwrap();

    let raw = db.message_trace(&msg.id).expect("trace stored");
    let back = RagTrace::from_json(&raw).expect("stored trace parses");
    assert_eq!(back, trace);
}

#[test]
fn message_trace_is_message_scoped() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("trace scope").unwrap();
    let a = db.add_message(&conv.id, "bot", "a", None).unwrap();
    let b = db.add_message(&conv.id, "bot", "b", None).unwrap();

    let mut trace = RagTrace::default();
    trace.push(step("agent", StepOutcome::Ok));
    db.set_message_trace(&a.id, &trace.to_json()).unwrap();

    assert!(db.message_trace(&a.id).is_some());
    assert_eq!(db.message_trace(&b.id), None);
}
