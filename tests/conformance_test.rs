// RFC 0012 T00 conformance corpus replay (device side). The SAME bytes live in marketplace-flowflow/tests
// and are replayed there through the backend gate; any divergence between the two gates fails an ordinary
// test. This is the parity mechanism in place of a shared crate (RFC 0012 §6.0): it pins the per-resource
// read_before_write semantics and the FSM structural validation across the two repos.

use flowflow::domain::governance::{
    gate, ConnectorManifest, Decision, Governance, ProposedCall, RunState,
};
use flowflow::domain::orchestration::Chain;
use serde_json::Value;

const CORPUS: &str = include_str!("conformance_vectors.json");

// The decision rendered as the corpus's expectation vocabulary: "allow", or the DenyReason's serde tag.
fn decision_tag(d: &Decision) -> String {
    match d {
        Decision::Allow => "allow".to_string(),
        Decision::Hold(_) => "hold".to_string(),
        Decision::Deny(reason) => serde_json::to_value(reason).unwrap()["deny"]
            .as_str()
            .unwrap()
            .to_string(),
    }
}

#[test]
fn gate_vectors_replay_identically() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    let conn: ConnectorManifest =
        serde_json::from_value(corpus["connector_manifest"].clone()).unwrap();

    for vec in corpus["gate_vectors"].as_array().unwrap() {
        let name = vec["name"].as_str().unwrap();
        let gov: Governance =
            serde_json::from_value(vec["governance"].clone()).unwrap();
        let mut run = RunState::default();
        // optional run_seed: steps/elapsed_seconds are caller-owned (the gate only reads them), so the
        // corpus seeds them to reach the step/time budgets.
        if let Some(seed) = vec.get("run_seed").and_then(|s| s.as_object()) {
            run.steps =
                seed.get("steps").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            run.elapsed_seconds = seed
                .get("elapsed_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            run.tool_calls =
                seed.get("tool_calls").and_then(|v| v.as_u64()).unwrap_or(0)
                    as u32;
        }
        for (i, c) in vec["calls"].as_array().unwrap().iter().enumerate() {
            let mut call = ProposedCall::new(
                c["tool"].as_str().unwrap(),
                c["args"].clone(),
            );
            if let Some(cols) = c.get("columns").and_then(|v| v.as_array()) {
                call = call.with_columns(
                    cols.iter().filter_map(|x| x.as_str().map(String::from)),
                );
            }
            let got = decision_tag(&gate(&gov, &conn, &call, &mut run));
            let want = c["expect"].as_str().unwrap();
            assert_eq!(
                got, want,
                "vector `{name}` call {i}: gate decision mismatch"
            );
        }
    }
}

#[test]
fn chain_vectors_validate_identically() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    for vec in corpus["chain_vectors"].as_array().unwrap() {
        let name = vec["name"].as_str().unwrap();
        let chain: Chain =
            serde_json::from_value(vec["chain"].clone()).unwrap();
        let ok = chain.validate().is_ok();
        let want_ok = vec["expect"].as_str().unwrap() == "ok";
        assert_eq!(
            ok, want_ok,
            "vector `{name}`: chain validate verdict mismatch"
        );
    }
}
