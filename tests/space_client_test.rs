// Space backend client wire shapes (proposal 0002, T08). serde silently drops
// a field it does not know, so a rename on the backend side would not fail a
// build here: it would land as a None at runtime, in the pull path, on device.
// These literals are the backend's Serialize structs
// (marketplace-flowflow/src/features/spaces/routes.rs), and they are the only
// thing that catches that drift.

use flowflow::infrastructure::backend::spaces::{IdResp, PullResp, SpaceResp};
use flowflow::infrastructure::backend::BackendError;

#[test]
fn pull_response_carries_the_tree_the_cursor_and_the_tombstones() {
    let raw = r#"{
        "folders": [{
            "id": "f1", "parent_id": null, "name": "Design",
            "mode": "collab", "effective_mode": "read",
            "author_ref": "abc123", "seq": 4,
            "updated_at": "2026-08-24T10:00:00Z", "deleted": false
        }],
        "notes": [{
            "id": "n1", "folder_id": "f1", "author_ref": "abc123",
            "own": false, "seq": 5, "updated_at": "2026-08-24T10:01:00Z",
            "deleted": true, "title": null, "content": null
        }],
        "next_seq": 5,
        "more": true
    }"#;
    let pull: PullResp = serde_json::from_str(raw).unwrap();

    let f = &pull.folders[0];
    assert_eq!(f.mode, "collab");
    // the declared mode is NOT the right to write: a read-only ancestor wins,
    // and only the server resolves that chain
    assert_eq!(f.effective_mode, "read");

    let n = &pull.notes[0];
    assert!(n.deleted);
    assert_eq!(n.title, None, "a tombstone keeps its row, not its content");
    assert!(!n.own);
    assert_eq!(pull.next_seq, 5);
    assert!(
        pull.more,
        "still catching up: pull again, skip the 30 s floor"
    );
}

#[test]
fn write_responses_carry_the_new_cursor() {
    let id: IdResp = serde_json::from_str(r#"{"id":"n9","seq":42}"#).unwrap();
    assert_eq!((id.id.as_str(), id.seq), ("n9", 42));

    let sp: SpaceResp =
        serde_json::from_str(r#"{"id":"s1","name":"Équipe"}"#).unwrap();
    assert_eq!(sp.name, "Équipe");
}

#[test]
fn a_dead_space_and_a_frozen_space_are_told_apart() {
    let gone = BackendError::Status(404, String::new());
    assert!(gone.is_not_found());
    assert!(!gone.is_read_only());

    // the owner stopped paying: pull keeps serving, writes refuse. Reading this
    // as "gone" would make the app drop a space that is only frozen.
    let frozen =
        BackendError::Status(409, r#"{"error":"space_read_only"}"#.into());
    assert!(frozen.is_read_only());
    assert!(!frozen.is_not_found());
    assert!(!frozen.is_limit());

    let capped =
        BackendError::Status(409, r#"{"error":"space_note_limit"}"#.into());
    assert!(capped.is_limit());
    assert!(!capped.is_read_only());
}
