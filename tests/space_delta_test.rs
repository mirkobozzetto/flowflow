// Space permission rule + delta application (proposal 0002, T09/T10).
//
// These two are the whole point of the feature and neither is visible from a
// type: "a read-only ancestor wins" is a walk up a chain, and "a pulled row
// overwrites the local copy, a tombstone deletes it" is a branch nobody sees
// fail until a member finds a ghost note in their chat.

use flowflow::application::space::apply_delta;
use flowflow::domain::space::{
    can_write_in, effective_mode, SpaceFolder, MODE_COLLAB, MODE_READ,
};
use flowflow::infrastructure::backend::spaces::{
    PullResp, RemoteFolder, RemoteSpaceNote,
};
use flowflow::infrastructure::persistence::Database;

const SPACE: &str = "space-1";

fn folder(id: &str, parent: Option<&str>, mode: &str) -> SpaceFolder {
    SpaceFolder {
        id: id.into(),
        parent_id: parent.map(String::from),
        mode: mode.into(),
    }
}

fn remote_folder(
    id: &str,
    parent: Option<&str>,
    name: &str,
    mode: &str,
) -> RemoteFolder {
    RemoteFolder {
        id: id.into(),
        parent_id: parent.map(String::from),
        name: name.into(),
        mode: mode.into(),
        effective_mode: mode.into(),
        author_ref: Some("author-a".into()),
        seq: 1,
        updated_at: "2026-08-24T10:00:00Z".into(),
        deleted: false,
    }
}

fn remote_note(
    id: &str,
    folder: Option<&str>,
    title: &str,
    content: &str,
) -> RemoteSpaceNote {
    RemoteSpaceNote {
        id: id.into(),
        folder_id: folder.map(String::from),
        author_ref: Some("author-a".into()),
        own: false,
        seq: 2,
        updated_at: "2026-08-24T10:00:00Z".into(),
        deleted: false,
        title: Some(title.into()),
        content: Some(content.into()),
    }
}

fn page(folders: Vec<RemoteFolder>, notes: Vec<RemoteSpaceNote>) -> PullResp {
    PullResp {
        folders,
        notes,
        next_seq: 9,
        more: false,
    }
}

fn db_with_space() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("flowflow.db")).unwrap();
    db.upsert_space(SPACE, "Team", false).unwrap();
    (dir, db)
}

// ---- the permission rule ----

#[test]
fn a_read_only_ancestor_restricts_the_whole_subtree() {
    let tree = vec![
        folder("root", None, MODE_READ),
        folder("child", Some("root"), MODE_COLLAB),
        folder("grandchild", Some("child"), MODE_COLLAB),
    ];
    // the child declares collab and is still read-only: the restriction
    // descends without a single child row being rewritten
    assert_eq!(effective_mode(&tree, "child"), MODE_READ);
    assert_eq!(effective_mode(&tree, "grandchild"), MODE_READ);

    let open = vec![
        folder("root", None, MODE_COLLAB),
        folder("child", Some("root"), MODE_COLLAB),
    ];
    assert_eq!(effective_mode(&open, "child"), MODE_COLLAB);
}

#[test]
fn an_unresolvable_chain_falls_to_read() {
    // parent deleted mid-pull: the safe side of a question we cannot answer
    let orphan = vec![folder("child", Some("vanished"), MODE_COLLAB)];
    assert_eq!(effective_mode(&orphan, "child"), MODE_READ);
    assert_eq!(effective_mode(&[], "nowhere"), MODE_READ);
}

#[test]
fn the_owner_writes_anywhere_and_a_member_never_at_the_root() {
    let tree = vec![
        folder("root", None, MODE_READ),
        folder("open", None, MODE_COLLAB),
    ];
    assert!(can_write_in(&tree, true, Some("root")));
    assert!(can_write_in(&tree, true, None));

    assert!(!can_write_in(&tree, false, Some("root")));
    assert!(can_write_in(&tree, false, Some("open")));
    // loose at the top of someone else's space: never
    assert!(!can_write_in(&tree, false, None));
}

// ---- applying a delta ----

#[test]
fn a_pulled_note_becomes_an_ordinary_local_note_in_its_folder() {
    let (_dir, db) = db_with_space();
    let out = apply_delta(
        &db,
        SPACE,
        &page(
            vec![remote_folder("rf1", None, "Design", MODE_COLLAB)],
            vec![remote_note("rn1", Some("rf1"), "Brief", "the brief body")],
        ),
    );
    assert_eq!((out.folders, out.notes, out.removed), (1, 1, 0));

    let local_folder = db.local_folder_for_remote(SPACE, "rf1").unwrap();
    let local_note = db.local_note_for_remote(SPACE, "rn1").unwrap();
    let note = db.get_note(&local_note).unwrap().unwrap();
    assert_eq!(note.content, "the brief body");
    assert_eq!(note.space_id.as_deref(), Some(SPACE));
    assert_eq!(note.author_ref.as_deref(), Some("author-a"));
    // an ordinary note in an ordinary folder: the rest of the app needs no
    // knowledge of spaces to list, search or chat with it
    let in_folder = db.list_notes_in_folder(&local_folder).unwrap();
    assert_eq!(in_folder.len(), 1);
    assert_eq!(in_folder[0].id, local_note);
}

#[test]
fn the_server_copy_overwrites_the_local_one() {
    let (_dir, db) = db_with_space();
    apply_delta(
        &db,
        SPACE,
        &page(vec![], vec![remote_note("rn1", None, "v1", "first body")]),
    );
    let local = db.local_note_for_remote(SPACE, "rn1").unwrap();

    let mut edited = remote_note("rn1", None, "v2", "second body");
    edited.seq = 7;
    apply_delta(&db, SPACE, &page(vec![], vec![edited]));

    // same local row, server content, no second note
    assert_eq!(
        db.local_note_for_remote(SPACE, "rn1").as_deref(),
        Some(local.as_str())
    );
    let note = db.get_note(&local).unwrap().unwrap();
    assert_eq!(note.content, "second body");
    assert_eq!(note.title.as_deref(), Some("v2"));
}

#[test]
fn a_tombstone_removes_the_local_note() {
    let (_dir, db) = db_with_space();
    apply_delta(
        &db,
        SPACE,
        &page(
            vec![],
            vec![remote_note("rn1", None, "doomed", "body here")],
        ),
    );
    let local = db.local_note_for_remote(SPACE, "rn1").unwrap();

    let mut dead = remote_note("rn1", None, "", "");
    dead.deleted = true;
    // a tombstone keeps its row, not its content
    dead.title = None;
    dead.content = None;
    let out = apply_delta(&db, SPACE, &page(vec![], vec![dead]));

    assert_eq!(out.removed, 1);
    assert!(db.get_note(&local).unwrap().is_none());
    assert!(db.local_note_for_remote(SPACE, "rn1").is_none());
}

#[test]
fn a_child_folder_finds_its_parent_in_the_same_page() {
    let (_dir, db) = db_with_space();
    // child FIRST: a single pass would hang it at the root and show a wrong tree
    apply_delta(
        &db,
        SPACE,
        &page(
            vec![
                remote_folder("child", Some("parent"), "Sub", MODE_COLLAB),
                remote_folder("parent", None, "Top", MODE_COLLAB),
            ],
            vec![],
        ),
    );

    let parent_local = db.local_folder_for_remote(SPACE, "parent").unwrap();
    let child_local = db.local_folder_for_remote(SPACE, "child").unwrap();
    let child = db.get_folder(&child_local).unwrap().unwrap();
    assert_eq!(child.parent_id.as_deref(), Some(parent_local.as_str()));

    // and the tree the write guard reads is keyed by REMOTE id, the one the
    // server reasons about
    let tree = db.space_folder_tree(SPACE).unwrap();
    let c = tree.iter().find(|f| f.id == "child").unwrap();
    assert_eq!(c.parent_id.as_deref(), Some("parent"));
    assert_eq!(effective_mode(&tree, "child"), MODE_COLLAB);
}

#[test]
fn a_deleted_folder_leaves_no_local_mirror() {
    let (_dir, db) = db_with_space();
    apply_delta(
        &db,
        SPACE,
        &page(
            vec![remote_folder("rf1", None, "Gone soon", MODE_COLLAB)],
            vec![],
        ),
    );
    let local = db.local_folder_for_remote(SPACE, "rf1").unwrap();

    let mut dead = remote_folder("rf1", None, "Gone soon", MODE_COLLAB);
    dead.deleted = true;
    apply_delta(&db, SPACE, &page(vec![dead], vec![]));

    assert!(db.get_folder(&local).unwrap().is_none());
    assert!(db.local_folder_for_remote(SPACE, "rf1").is_none());
}

// ---- pull cadence (T13) ----

#[test]
fn the_thirty_second_floor_holds_between_two_pulls() {
    use chrono::{Duration, Utc};
    use flowflow::domain::space::due_for_pull;

    let now = Utc::now();
    // never pulled: always due, or a joined space would show empty until the
    // floor elapsed
    assert!(due_for_pull(None, now));

    let just_now = (now - Duration::seconds(5)).to_rfc3339();
    assert!(!due_for_pull(Some(&just_now), now));

    let stale = (now - Duration::seconds(31)).to_rfc3339();
    assert!(due_for_pull(Some(&stale), now));

    // a stamp we cannot read is not a reason to stop refreshing
    assert!(due_for_pull(Some("not a date"), now));
}
