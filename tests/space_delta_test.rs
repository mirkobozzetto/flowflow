// Space permission rule + delta application (proposal 0002, T09/T10).
//
// These two are the whole point of the feature and neither is visible from a
// type: "a read-only ancestor wins" is a walk up a chain, and "a pulled row
// overwrites the local copy, a tombstone deletes it" is a branch nobody sees
// fail until a member finds a ghost note in their chat.

use flowflow::application::space::{apply_delta, PullOutcome};
use flowflow::domain::space::{
    can_write_in, effective_mode, SpaceFolder, MODE_COLLAB, MODE_READ,
};
use flowflow::infrastructure::backend::spaces::{
    PullResp, RemoteFolder, RemoteSpaceNote, RemoteThread,
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
        thread_id: None,
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
        threads: vec![],
        next_seq: 9,
        more: false,
    }
}

// The real path: one page = one transaction, cursor included.
fn apply(db: &Database, p: &PullResp) -> PullOutcome {
    db.apply_space_page(SPACE, p.next_seq, |tx| apply_delta(tx, SPACE, p))
        .expect("page applies")
        .0
}

fn cursor(db: &Database) -> i64 {
    db.get_space(SPACE).unwrap().unwrap().cursor
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
    let out = apply(
        &db,
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
    // a pulled thread lands under its own id and its member note joins it;
    // a tombstoned thread detaches the note without deleting it
    let thread = |deleted: bool| RemoteThread {
        id: "rt1".into(),
        folder_id: Some("rf1".into()),
        title: "Brief thread".into(),
        author_ref: Some("author-a".into()),
        own: false,
        seq: 3,
        updated_at: "2026-08-24T10:00:00Z".into(),
        deleted,
    };
    let mut member = remote_note("rn1", Some("rf1"), "Brief", "the brief body");
    member.thread_id = Some("rt1".into());
    let mut p = page(vec![], vec![member.clone()]);
    p.threads = vec![thread(false)];
    p.next_seq = 10;
    let out = apply(&db, &p);
    assert_eq!(out.threads, 1);
    let note = db.get_note(&local_note).unwrap().unwrap();
    assert_eq!(note.thread_id.as_deref(), Some("rt1"));
    assert_eq!(db.get_thread("rt1").unwrap().unwrap().title, "Brief thread");

    member.thread_id = None;
    let mut p = page(vec![], vec![member]);
    p.threads = vec![thread(true)];
    p.next_seq = 11;
    apply(&db, &p);
    let note = db.get_note(&local_note).unwrap().unwrap();
    assert_eq!(note.thread_id, None);
    assert!(db.get_thread("rt1").unwrap().is_none());
}

#[test]
fn the_server_copy_overwrites_the_local_one() {
    let (_dir, db) = db_with_space();
    apply(
        &db,
        &page(vec![], vec![remote_note("rn1", None, "v1", "first body")]),
    );
    let local = db.local_note_for_remote(SPACE, "rn1").unwrap();

    let mut edited = remote_note("rn1", None, "v2", "second body");
    edited.seq = 7;
    apply(&db, &page(vec![], vec![edited]));

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
    apply(
        &db,
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
    let out = apply(&db, &page(vec![], vec![dead]));

    assert_eq!(out.removed, 1);
    assert!(db.get_note(&local).unwrap().is_none());
    assert!(db.local_note_for_remote(SPACE, "rn1").is_none());
}

#[test]
fn a_child_folder_finds_its_parent_in_the_same_page() {
    let (_dir, db) = db_with_space();
    // child FIRST: a single pass would hang it at the root and show a wrong tree
    apply(
        &db,
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
    apply(
        &db,
        &page(
            vec![remote_folder("rf1", None, "Gone soon", MODE_COLLAB)],
            vec![],
        ),
    );
    let local = db.local_folder_for_remote(SPACE, "rf1").unwrap();

    let mut dead = remote_folder("rf1", None, "Gone soon", MODE_COLLAB);
    dead.deleted = true;
    apply(&db, &page(vec![dead], vec![]));

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

// ---- the write right the UI reads (T12) ----

#[test]
fn the_ui_reads_the_same_right_the_server_enforces() {
    use flowflow::application::space::{folder_right, FolderRight};

    let (_dir, db) = db_with_space();
    apply(
        &db,
        &page(
            vec![
                remote_folder("open", None, "Open", MODE_COLLAB),
                remote_folder("locked", None, "Locked", MODE_READ),
                remote_folder("under", Some("locked"), "Under", MODE_COLLAB),
            ],
            vec![],
        ),
    );
    let local = |r: &str| db.local_folder_for_remote(SPACE, r).unwrap();

    assert_eq!(
        folder_right(&db, &local("open")),
        FolderRight::SpaceWritable
    );
    assert_eq!(
        folder_right(&db, &local("locked")),
        FolderRight::SpaceReadOnly
    );
    // declares collab, sits under a read-only parent: the badge and the
    // compose button must both say no
    assert_eq!(
        folder_right(&db, &local("under")),
        FolderRight::SpaceReadOnly
    );

    // an ordinary theme is never touched by any of this
    let plain = db
        .create_folder(&flowflow::domain::NewFolder {
            name: "Mine".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    assert_eq!(folder_right(&db, &plain.id), FolderRight::Local);
}

// ---- invite links ----

#[test]
fn a_space_link_is_accepted_with_or_without_its_prefix() {
    use flowflow::domain::space::{parse_space_link, space_link};

    let code = "Ab3-_xYz01234567";
    let link = space_link(code);
    assert_eq!(link, format!("flowflow://space/{code}"));
    assert_eq!(parse_space_link(&link), Some(code));
    // pasted bare out of a chat message: same intent, refusing it teaches
    // nothing
    assert_eq!(parse_space_link(code), Some(code));
    assert_eq!(parse_space_link(&format!("  {link}  ")), Some(code));

    // a share link is NOT a space link: it opens a snapshot, it does not join
    assert_eq!(parse_space_link("flowflow://share/abc"), None);
    assert_eq!(parse_space_link(""), None);
}

// ---- a page is atomic, and so is its cursor (proposal 0003) ----

// The rows the server sends are valid; what fails mid-page is the disk, an FK,
// a busy lock. A trigger stands in for that: it aborts the second note of the
// page, and nothing of the page may survive, the cursor least of all.
#[test]
fn a_page_that_fails_midway_leaves_no_row_and_no_cursor() {
    let (_dir, db) = db_with_space();
    apply(
        &db,
        &page(
            vec![remote_folder("keep", None, "Kept", MODE_COLLAB)],
            vec![],
        ),
    );
    assert_eq!(cursor(&db), 9);
    db.conn()
        .execute_batch(
            "CREATE TRIGGER abort_boom BEFORE INSERT ON notes
             WHEN NEW.title = 'boom'
             BEGIN SELECT RAISE(ABORT, 'disk said no'); END;",
        )
        .unwrap();

    let mut p = page(
        vec![remote_folder("rf1", None, "Design", MODE_COLLAB)],
        vec![
            remote_note("rn1", Some("rf1"), "fine", "lands first"),
            remote_note("rn2", Some("rf1"), "boom", "aborts the page"),
        ],
    );
    p.next_seq = 42;
    let err = db
        .apply_space_page(SPACE, p.next_seq, |tx| apply_delta(tx, SPACE, &p))
        .expect_err("the page must fail");
    assert!(err.contains("disk said no"), "the cause surfaces: {err}");

    // nothing of the page landed: not the folder, not the note that had
    // already been inserted before the abort
    assert!(db.local_folder_for_remote(SPACE, "rf1").is_none());
    assert!(db.local_note_for_remote(SPACE, "rn1").is_none());
    assert_eq!(db.list_notes().unwrap().len(), 0);
    // and the cursor still points at the page, so the server replays it
    assert_eq!(cursor(&db), 9);
    // the earlier page is untouched
    assert!(db.local_folder_for_remote(SPACE, "keep").is_some());

    // once the cause is gone, the same page applies whole
    db.conn().execute_batch("DROP TRIGGER abort_boom").unwrap();
    let out = apply(&db, &p);
    assert_eq!((out.folders, out.notes), (1, 2));
    assert_eq!(cursor(&db), 42);
}

// Every repo call a page can make runs under the page transaction, including
// the ones that used to open a transaction of their own: a delete, an unlink.
// A BEGIN inside a BEGIN fails, and this is the test that would catch it.
#[test]
fn a_full_page_with_deletes_and_moves_commits_as_one() {
    let (_dir, db) = db_with_space();
    apply(
        &db,
        &page(
            vec![
                remote_folder("a", None, "A", MODE_COLLAB),
                remote_folder("b", None, "B", MODE_COLLAB),
                remote_folder("gone", None, "Gone", MODE_COLLAB),
            ],
            vec![
                remote_note("n1", Some("a"), "one", "moves to b"),
                remote_note("n2", Some("a"), "two", "gets deleted"),
            ],
        ),
    );
    let n1 = db.local_note_for_remote(SPACE, "n1").unwrap();
    let a = db.local_folder_for_remote(SPACE, "a").unwrap();
    let b = db.local_folder_for_remote(SPACE, "b").unwrap();

    let mut dead_note = remote_note("n2", None, "", "");
    dead_note.deleted = true;
    dead_note.title = None;
    dead_note.content = None;
    let mut dead_folder = remote_folder("gone", None, "Gone", MODE_COLLAB);
    dead_folder.deleted = true;
    let mut p = page(
        vec![dead_folder],
        vec![remote_note("n1", Some("b"), "one", "moved"), dead_note],
    );
    p.next_seq = 20;
    let out = apply(&db, &p);
    assert_eq!((out.folders, out.notes, out.removed), (1, 1, 1));
    assert_eq!(cursor(&db), 20);

    assert!(db.local_folder_for_remote(SPACE, "gone").is_none());
    assert!(db.local_note_for_remote(SPACE, "n2").is_none());
    let folders: Vec<String> = db
        .folders_for_note(&n1)
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(folders, vec![b.clone()], "relinked from {a} to {b}");
    // and the connection is free again: an ordinary call after the page
    assert!(db.get_note(&n1).unwrap().is_some());
}
