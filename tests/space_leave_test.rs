// Leaving or being revoked (proposal 0002, T14).
//
// The local half is what matters here: what the departing member keeps, and
// what must not survive. Someone else's note left indexed on a device that no
// longer reads the space is the ghost note the whole design exists to prevent,
// and a kept note that still waits for a pull is a note that will never update
// again.

use flowflow::application::space::{detach_locally, Departure};
use flowflow::infrastructure::backend::spaces::{PullResp, RemoteSpaceNote};
use flowflow::infrastructure::persistence::Database;

const SPACE: &str = "space-1";
const ME: &str = "author-me";
const THEM: &str = "author-them";

fn note(id: &str, author: &str, own: bool) -> RemoteSpaceNote {
    RemoteSpaceNote {
        id: id.into(),
        folder_id: None,
        thread_id: None,
        author_ref: Some(author.into()),
        own,
        seq: 1,
        updated_at: "2026-08-24T10:00:00Z".into(),
        deleted: false,
        title: Some(id.into()),
        content: Some(format!("body of {id}")),
    }
}

fn joined_space_with_two_notes() -> (tempfile::TempDir, Database, String, String)
{
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("flowflow.db")).unwrap();
    db.upsert_space(SPACE, "Team", false).unwrap();
    let page = PullResp {
        folders: vec![],
        notes: vec![note("mine", ME, true), note("theirs", THEM, false)],
        threads: vec![],
        next_seq: 2,
        more: false,
    };
    db.apply_space_page(SPACE, page.next_seq, |tx| {
        flowflow::application::space::apply_delta(tx, SPACE, &page)
    })
    .unwrap();
    let mine = db.local_note_for_remote(SPACE, "mine").unwrap();
    let theirs = db.local_note_for_remote(SPACE, "theirs").unwrap();
    (dir, db, mine, theirs)
}

#[test]
fn keeping_my_notes_turns_them_into_ordinary_local_ones() {
    let (_d, db, mine, theirs) = joined_space_with_two_notes();

    detach_locally(&db, SPACE, Departure::KeepMine);

    let kept = db.get_note(&mine).unwrap().expect("my note stays");
    // no space, no remote id: it waits for no pull and will never be
    // overwritten again. Its local id did not change, so its embeddings stay
    // valid and nothing needs re-purging.
    assert_eq!(kept.space_id, None);
    assert_eq!(kept.content, "body of mine");
    assert!(db.local_note_for_remote(SPACE, "mine").is_none());

    // someone else's note goes: keeping it indexed on a device that no longer
    // reads the space is exactly the ghost note the design forbids
    assert!(db.get_note(&theirs).unwrap().is_none());
    assert!(db.list_spaces().unwrap().is_empty());
}

#[test]
fn withdrawing_leaves_nothing_behind_locally() {
    let (_d, db, mine, theirs) = joined_space_with_two_notes();

    detach_locally(&db, SPACE, Departure::WithdrawMine);

    assert!(db.get_note(&mine).unwrap().is_none());
    assert!(db.get_note(&theirs).unwrap().is_none());
    // and each deletion queued its vector purge, so no orphan vector answers
    // in chat afterwards
    let queued = db.pending_purges().unwrap();
    assert!(queued.iter().any(|(id, _)| id == &mine));
    assert!(queued.iter().any(|(id, _)| id == &theirs));
}

#[test]
fn my_author_handle_is_learned_from_my_own_pulled_notes() {
    let (_d, db, _, _) = joined_space_with_two_notes();
    // the pull payload never states it outright; the `own` flag does
    assert_eq!(
        flowflow::application::space::my_author_ref(&db).as_deref(),
        Some(ME)
    );
}

// Revoked owner-side: the pull answers the uniform 404 and the mirror must go.
// Nothing is destroyed though - proposal §6.6, only the departing author may
// ever withdraw content, and this device did not choose to leave.
#[test]
fn being_revoked_keeps_every_note_as_an_ordinary_one() {
    let (_d, db, mine, theirs) = joined_space_with_two_notes();

    detach_locally(&db, SPACE, Departure::Revoked);

    for id in [&mine, &theirs] {
        let note = db.get_note(id).unwrap().expect("nothing is destroyed");
        assert_eq!(note.space_id, None, "it waits for no pull any more");
    }
    assert!(db.pending_purges().unwrap().is_empty(), "no vector purged");
    assert!(db.list_spaces().unwrap().is_empty(), "the mirror is gone");
}
