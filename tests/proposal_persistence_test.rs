// The approval card's persistence contract (RFC 0019 T08): a proposal is written at propose
// time as a role="proposal" row in its pending shape, the decision rewrites that same row in
// place, and a reload maps a still-pending row (dead run) to Expired while unknown roles drop.

use flowflow::application::approvals::{
    is_renderable_role, reload_proposal, PersistedProposal, ProposalView,
    ReloadStatus,
};
use flowflow::infrastructure::persistence::Database;
use serde_json::json;
use std::sync::Once;
use tokio::sync::Mutex;

static DB_LOCK: Mutex<()> = Mutex::const_new(());
static SEAM: Once = Once::new();

fn isolate_store() {
    SEAM.call_once(|| {
        let dir = std::env::temp_dir()
            .join(format!("flowflow-test-proposals-{}", std::process::id()));
        std::env::set_var("FLOWFLOW_DATA_DIR", &dir);
        std::env::set_var("FLOWFLOW_VECTORDB_PATH", dir.join("vectordb"));
    });
}

fn sample_view(id: &str) -> ProposalView {
    ProposalView {
        id: id.to_string(),
        tool: "google_sheets_write_to_cell".into(),
        action: "update".into(),
        rows: vec![
            ("cell".into(), "B2".into()),
            ("value".into(), "devis vendredi".into()),
        ],
        raw_args: json!({"cell": "B2", "value": "devis vendredi"}),
    }
}

#[tokio::test]
async fn proposal_row_persists_at_propose_shape_then_updates_in_place() {
    isolate_store();
    let _guard = DB_LOCK.lock().await;
    let db = Database::open().expect("open db");
    let conv = db.create_conversation("approval test").expect("conv");

    let view = sample_view("11111111-1111-1111-1111-111111111111");
    let pending = serde_json::to_string(&PersistedProposal {
        view: view.clone(),
        status: "pending".to_string(),
    })
    .unwrap();
    let msg = db
        .add_message(&conv.id, "proposal", &pending, None)
        .expect("add proposal");

    // Propose-time shape: role "proposal", the view round-trips, pending reloads as Expired.
    let listed = db.list_messages(&conv.id).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].role, "proposal");
    let (rv, rs) = reload_proposal(&listed[0].content).expect("parse pending");
    assert_eq!(rv, view);
    assert_eq!(rs, ReloadStatus::Expired);

    // The decision rewrites the SAME row, no second message.
    let approved = serde_json::to_string(&PersistedProposal {
        view: view.clone(),
        status: "approved".to_string(),
    })
    .unwrap();
    db.update_message_content(&msg.id, &approved)
        .expect("update content");

    let listed = db.list_messages(&conv.id).expect("list");
    assert_eq!(listed.len(), 1);
    let (_, rs) = reload_proposal(&listed[0].content).expect("parse approved");
    assert_eq!(rs, ReloadStatus::Approved);

    db.delete_conversation(&conv.id).expect("cleanup");
}

#[test]
fn reload_maps_pending_and_unknown_status_to_expired() {
    let pending = serde_json::to_string(&PersistedProposal {
        view: sample_view("x"),
        status: "pending".to_string(),
    })
    .unwrap();
    assert_eq!(reload_proposal(&pending).unwrap().1, ReloadStatus::Expired);

    let junk = serde_json::to_string(&PersistedProposal {
        view: sample_view("x"),
        status: "weird".to_string(),
    })
    .unwrap();
    assert_eq!(reload_proposal(&junk).unwrap().1, ReloadStatus::Expired);

    // Content that is not a persisted proposal is dropped, never rendered as garbage.
    assert!(reload_proposal("not a proposal").is_none());
}

#[test]
fn unknown_roles_are_dropped_on_reload() {
    assert!(is_renderable_role("user"));
    assert!(is_renderable_role("bot"));
    assert!(is_renderable_role("proposal"));
    assert!(!is_renderable_role("system"));
    assert!(!is_renderable_role(""));
}
