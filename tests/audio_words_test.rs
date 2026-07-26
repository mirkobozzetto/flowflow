use flowflow::domain::{NewTextNote, Transcript, Word};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");
    (db, dir)
}

fn note_with_audio(db: &Database) -> (String, String) {
    let note = db
        .create_text_note(&NewTextNote {
            title: Some("note".into()),
            content: String::new(),
            tags: vec![],
        })
        .expect("create note");
    let audio = db.add_audio(&note.id, "clip.wav", 12.5).expect("add audio");
    (note.id, audio.id)
}

fn sample() -> Transcript {
    Transcript::new(vec![
        Word::new("Bonjour", 0, 500, 0.98),
        Word::new("le", 500, 700, 0.91),
        Word::new("monde", 700, 1200, 0.95),
    ])
}

fn word_row_count(db: &Database) -> i64 {
    db.conn()
        .query_row("SELECT COUNT(*) FROM note_audio_words", [], |r| r.get(0))
        .expect("count")
}

#[test]
fn migration_v20_creates_the_sidecar_table() {
    let (db, _dir) = open_test_db();
    assert_eq!(word_row_count(&db), 0);
}

#[test]
fn a_transcript_round_trips_through_the_sidecar() {
    let (db, _dir) = open_test_db();
    let (_note, audio) = note_with_audio(&db);

    db.set_audio_transcript(&audio, &sample()).expect("write");

    let back = db.words_for_audio(&audio).expect("word row");
    assert_eq!(back, sample());
}

#[test]
fn the_stored_text_is_derived_from_the_words() {
    let (db, _dir) = open_test_db();
    let (note, audio) = note_with_audio(&db);

    db.set_audio_transcript(&audio, &sample()).expect("write");

    let audios = db.list_audios(&note).expect("list");
    assert_eq!(audios[0].transcription.as_deref(), Some("Bonjour le monde"));
}

/// A transcript with no timings must not leave a word row behind: taps built on
/// all-zero timings would every one of them seek to the start of the clip.
#[test]
fn an_untimed_transcript_clears_any_previous_word_row() {
    let (db, _dir) = open_test_db();
    let (_note, audio) = note_with_audio(&db);

    db.set_audio_transcript(&audio, &sample()).expect("write");
    assert_eq!(word_row_count(&db), 1);

    db.set_audio_transcript(&audio, &Transcript::from_text("texte dicté"))
        .expect("overwrite");
    assert_eq!(word_row_count(&db), 0);
    assert!(db.words_for_audio(&audio).is_none());
}

#[test]
fn an_audio_without_words_reads_back_as_absent() {
    let (db, _dir) = open_test_db();
    let (_note, audio) = note_with_audio(&db);
    assert!(db.words_for_audio(&audio).is_none());
}

// The four paths that remove a note_audios row. The sidecar has no foreign key
// (migration V20 explains why), so each one is asserted rather than assumed.

#[test]
fn delete_audio_takes_its_words_with_it() {
    let (db, _dir) = open_test_db();
    let (_note, audio) = note_with_audio(&db);
    db.set_audio_transcript(&audio, &sample()).expect("write");

    db.delete_audio(&audio).expect("delete audio");
    assert_eq!(word_row_count(&db), 0);
}

#[test]
fn deleting_the_note_takes_its_audio_words_with_it() {
    let (db, _dir) = open_test_db();
    let (note, audio) = note_with_audio(&db);
    db.set_audio_transcript(&audio, &sample()).expect("write");

    db.delete_note(&note).expect("delete note");
    assert_eq!(word_row_count(&db), 0);
}

#[test]
fn delete_all_audios_takes_every_word_row_with_it() {
    let (db, dir) = open_test_db();
    let (_note, audio) = note_with_audio(&db);
    db.set_audio_transcript(&audio, &sample()).expect("write");

    db.delete_all_audios(dir.path().to_str().expect("dir"))
        .expect("delete all");
    assert_eq!(word_row_count(&db), 0);
}

/// "Delete my data" (RFC 0009). A transcript that survives an explicit wipe is a
/// privacy defect, not an orphan nuisance.
#[test]
fn wipe_local_content_leaves_no_word_row_behind() {
    let (db, _dir) = open_test_db();
    let (_note, audio) = note_with_audio(&db);
    db.set_audio_transcript(&audio, &sample()).expect("write");

    db.wipe_local_content().expect("wipe");
    assert_eq!(word_row_count(&db), 0);
}
