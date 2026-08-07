use flowflow::application::note_persistence::commit_transcription;
use flowflow::domain::NewTextNote;
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");
    (db, dir)
}

fn note_with(db: &Database, content: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("titre".into()),
        content: content.to_string(),
        tags: vec!["a".into(), "b".into()],
    })
    .expect("create note")
    .id
}

fn body(db: &Database, id: &str) -> String {
    db.get_note(id).expect("get").expect("some").content
}

fn modified_at(db: &Database, id: &str) -> String {
    db.get_note(id).expect("get").expect("some").modified_at
}

#[test]
fn an_empty_note_takes_the_transcript_as_its_body() {
    let (db, _dir) = open_test_db();
    let id = note_with(&db, "");

    let committed =
        commit_transcription(&db, &id, "", "bonjour").expect("committed");

    assert_eq!(committed.merged, "bonjour");
    assert_eq!(body(&db, &id), "bonjour");
}

#[test]
fn an_existing_body_gets_the_transcript_on_a_new_line() {
    let (db, _dir) = open_test_db();
    let id = note_with(&db, "déjà là");

    commit_transcription(&db, &id, "déjà là", "suite").expect("committed");

    assert_eq!(body(&db, &id), "déjà là\nsuite");
}

#[test]
fn two_commits_append_twice_instead_of_replacing() {
    let (db, _dir) = open_test_db();
    let id = note_with(&db, "");

    commit_transcription(&db, &id, "", "un").expect("first");
    let current = body(&db, &id);
    commit_transcription(&db, &id, &current, "deux").expect("second");

    assert_eq!(body(&db, &id), "un\ndeux");
}

#[test]
fn a_blank_transcript_writes_nothing_and_leaves_modified_at_alone() {
    let (db, _dir) = open_test_db();
    let id = note_with(&db, "texte réel");
    let before = modified_at(&db, &id);

    assert!(commit_transcription(&db, &id, "texte réel", "  \n ").is_none());

    assert_eq!(body(&db, &id), "texte réel");
    assert_eq!(
        modified_at(&db, &id),
        before,
        "an empty edit must not win a later sync merge"
    );
}

#[test]
fn an_unknown_note_commits_nothing() {
    let (db, _dir) = open_test_db();

    assert!(commit_transcription(&db, "nope", "", "bonjour").is_none());
    assert!(db.get_note("nope").expect("get").is_none());
}

#[test]
fn the_open_editor_body_wins_over_the_stored_one() {
    let (db, _dir) = open_test_db();
    let id = note_with(&db, "en base");

    commit_transcription(&db, &id, "dans l'éditeur", "suite")
        .expect("committed");

    assert_eq!(body(&db, &id), "dans l'éditeur\nsuite");
}

#[test]
fn the_committed_body_carries_what_embedding_needs() {
    let (db, _dir) = open_test_db();
    let id = note_with(&db, "");
    let created_at = db.get_note(&id).expect("get").expect("some").created_at;

    let committed =
        commit_transcription(&db, &id, "", "bonjour").expect("committed");

    assert_eq!(committed.title, "titre");
    assert_eq!(committed.tags, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(committed.created_at, created_at);
}
