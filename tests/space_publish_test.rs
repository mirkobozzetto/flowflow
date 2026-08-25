// Publishing a note saved into a space (proposal 0003 T06-T08).
//
// A note the editor saved into a space folder must end up on the server once,
// or visibly become a local note again. The staging row is what carries that
// promise across an offline save, a kill and a restart; these tests hold the
// row's contract, the retry rule and the drain at the head of a pull. The push
// itself needs a backend and is validated on device.

use flowflow::application::space::republish_pending;
use flowflow::domain::space::{
    publish_backoff, PUBLISH_RETRY_BASE_SECS, PUBLISH_RETRY_MAX_SECS,
};
use flowflow::domain::NewFolder;
use flowflow::infrastructure::persistence::{now_iso, Database};

const SPACE: &str = "space-1";

fn db_with_space_folder() -> (tempfile::TempDir, Database, String) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("flowflow.db")).unwrap();
    db.upsert_space(SPACE, "Team", true).unwrap();
    let folder = db
        .create_folder(&NewFolder {
            name: "Shared".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    db.mark_folder_in_space(&folder.id, SPACE, "rf1", "collab")
        .unwrap();
    (dir, db, folder.id)
}

fn local_note(db: &Database, folder: &str, body: &str) -> String {
    flowflow::application::note_persistence::create_note(
        db,
        "",
        body,
        vec![],
        Some(folder),
        None,
    )
    .unwrap()
    .0
    .id
}

#[test]
fn staging_binds_the_remote_id_and_queues_the_note_as_one() {
    let (_d, db, folder) = db_with_space_folder();
    let id = local_note(&db, &folder, "saved offline");

    db.stage_note_publish(&id, SPACE, "remote-1", None).unwrap();

    let note = db.get_note(&id).unwrap().unwrap();
    assert_eq!(note.remote_id.as_deref(), Some("remote-1"));
    assert_eq!(note.space_id.as_deref(), Some(SPACE));
    let pending = db.note_publish_state(&id).expect("queued");
    assert_eq!((pending.attempts, pending.space_id.as_str()), (0, SPACE));
    assert!(pending.next_try_at <= now_iso(), "due at once");

    // staging again keeps the row: a second save does not reset the retries
    db.defer_note_publish(&id, "offline").unwrap();
    db.stage_note_publish(&id, SPACE, "remote-1", None).unwrap();
    assert_eq!(db.note_publish_state(&id).unwrap().attempts, 1);

    // a note that is gone owes nothing
    flowflow::application::note_persistence::delete_note(&db, &id);
    assert!(db.note_publish_state(&id).is_none());
}

#[test]
fn a_deferred_note_waits_out_its_backoff() {
    let (_d, db, folder) = db_with_space_folder();
    let id = local_note(&db, &folder, "body");
    db.stage_note_publish(&id, SPACE, "remote-1", None).unwrap();
    assert_eq!(
        db.due_note_publishes(SPACE, &now_iso(), 20).unwrap(),
        vec![id.clone()]
    );

    db.defer_note_publish(&id, "network: down").unwrap();
    let p = db.note_publish_state(&id).unwrap();
    assert_eq!(p.attempts, 1);
    assert_eq!(p.last_error.as_deref(), Some("network: down"));
    assert!(p.next_try_at > now_iso(), "pushed into the future");
    assert!(db
        .due_note_publishes(SPACE, &now_iso(), 20)
        .unwrap()
        .is_empty());

    // a never-staged note is left alone
    db.defer_note_publish("nobody", "x").unwrap();
    assert!(db.note_publish_state("nobody").is_none());

    // the rule: 1 min, 2, 4 ... capped at an hour
    assert_eq!(publish_backoff(0).num_seconds(), PUBLISH_RETRY_BASE_SECS);
    assert_eq!(
        publish_backoff(1).num_seconds(),
        2 * PUBLISH_RETRY_BASE_SECS
    );
    assert_eq!(
        publish_backoff(5).num_seconds(),
        32 * PUBLISH_RETRY_BASE_SECS
    );
    assert_eq!(publish_backoff(6).num_seconds(), PUBLISH_RETRY_MAX_SECS);
    assert_eq!(publish_backoff(40).num_seconds(), PUBLISH_RETRY_MAX_SECS);
}

#[test]
fn the_drain_takes_the_due_notes_only_and_at_most_twenty() {
    let (_d, db, folder) = db_with_space_folder();
    let mut ids = Vec::new();
    for i in 0..25 {
        let id = local_note(&db, &folder, &format!("note {i}"));
        db.stage_note_publish(&id, SPACE, &format!("r-{i}"), None)
            .unwrap();
        ids.push(id);
    }
    // one of them already failed and is not due
    db.defer_note_publish(&ids[3], "later").unwrap();
    // a note of another space is not this pull's business
    let other = local_note(&db, &folder, "elsewhere");
    db.stage_note_publish(&other, "space-2", "r-x", None)
        .unwrap();

    let due = db.due_note_publishes(SPACE, &now_iso(), 20).unwrap();
    assert_eq!(due.len(), 20, "capped");
    assert!(!due.contains(&ids[3]), "next_try_at honored");
    assert!(!due.contains(&other), "scoped to the space");

    // With no backend configured every push fails locally: the drain tries
    // the twenty due notes, defers each, and a second drain finds none due.
    let rt = tokio::runtime::Runtime::new().unwrap();
    assert_eq!(rt.block_on(republish_pending(&db, SPACE)), 20);
    let p = db.note_publish_state(&ids[0]).unwrap();
    assert_eq!(p.attempts, 1);
    assert!(p.last_error.is_some());
    // the four left over are taken next, then nothing is due until the backoff
    assert_eq!(rt.block_on(republish_pending(&db, SPACE)), 4);
    assert_eq!(rt.block_on(republish_pending(&db, SPACE)), 0);
    // still queued, none detached: a local failure is never the server's verdict
    assert_eq!(
        db.get_note(&ids[0]).unwrap().unwrap().space_id.as_deref(),
        Some(SPACE)
    );
    assert_eq!(
        db.get_note(&ids[24]).unwrap().unwrap().remote_id.as_deref(),
        Some("r-24")
    );
}
