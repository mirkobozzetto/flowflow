// Replayable vector purge (proposal 0002, T11/T11b).
//
// SQLite and LanceDB share no transaction. Deleting a note removes the SQLite
// row at once while the vector delete can fail, and a vector that outlives its
// note keeps answering in chat - the exact defect the design forbids. The queue
// is what turns "we called delete" into "the delete happened", so what needs
// proving is that the INTENT is recorded on every delete path, LanceDB present
// or not.

use flowflow::domain::NewTextNote;
use flowflow::infrastructure::persistence::Database;

fn db() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("flowflow.db")).unwrap();
    (dir, db)
}

fn make_note(db: &Database, content: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("t".into()),
        content: content.into(),
        tags: vec![],
    })
    .unwrap()
    .id
}

#[test]
fn deleting_a_note_records_the_purge_intent() {
    let (_d, db) = db();
    let id = make_note(&db, "something worth embedding, long enough");

    flowflow::application::note_persistence::delete_note(&db, &id);

    assert!(db.get_note(&id).unwrap().is_none());
    let pending = db.pending_purges().unwrap();
    assert!(
        pending.contains(&(id.clone(), "note".to_string())),
        "the SQLite row is gone; without a queued purge the vector would \
         survive it, {pending:?}"
    );
}

#[test]
fn the_queue_survives_until_the_vector_store_confirms() {
    let (_d, db) = db();
    db.queue_purge("n1", "note").unwrap();
    // queued twice (a retry, a second delete signal): still one row
    db.queue_purge("n1", "note").unwrap();
    assert_eq!(db.pending_purges().unwrap().len(), 1);

    // only a confirmed delete clears it
    db.clear_purge("n1", "note").unwrap();
    assert!(db.pending_purges().unwrap().is_empty());
}

#[test]
fn a_note_and_an_attachment_queue_separately() {
    let (_d, db) = db();
    db.queue_purge("x", "note").unwrap();
    db.queue_purge("x", "attachment").unwrap();
    assert_eq!(db.pending_purges().unwrap().len(), 2);

    db.clear_purge("x", "note").unwrap();
    assert_eq!(
        db.pending_purges().unwrap(),
        vec![("x".to_string(), "attachment".to_string())]
    );
}

// T11b: the P2P applier deletes the SQLite chunks but cannot reach LanceDB from
// inside a sync transaction. Before this, a note deleted on one device and
// echoed here left its vector alive on this one.
#[test]
fn the_p2p_delete_path_queues_the_purge_too() {
    let (_d, db) = db();
    let id = make_note(&db, "arrived from a peer, then deleted there");

    let applied = {
        let conn = db.conn();
        flowflow::infrastructure::sync::protocol::apply_entity_delete_for_test(
            &conn, "note", &id,
        )
    };
    applied.unwrap();

    assert!(db.get_note(&id).unwrap().is_none());
    assert!(
        db.pending_purges()
            .unwrap()
            .contains(&(id.clone(), "note".to_string())),
        "a P2P deletion must leave the same purge intent as a local one"
    );
}
