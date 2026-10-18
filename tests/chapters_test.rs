use flowflow::application::chapters::parse_chapters;
use flowflow::domain::reading::Chapter;
use flowflow::domain::{NewTextNote, Transcript, Word};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");
    (db, dir)
}

fn transcribed_clip(db: &Database, words: &[&str]) -> String {
    let note = db
        .create_text_note(&NewTextNote {
            title: Some("note".into()),
            content: String::new(),
            tags: vec![],
        })
        .expect("create note");
    let audio = db.add_audio(&note.id, "clip.wav", 12.5).expect("add audio");
    db.set_audio_transcript(&audio.id, &transcript(words))
        .expect("set transcript");
    audio.id
}

fn transcript(words: &[&str]) -> Transcript {
    Transcript::new(
        words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let at = i as u32 * 500;
                Word::new(*w, at, at + 400, 0.9)
            })
            .collect(),
    )
}

fn chapter(start_word: usize, title: &str) -> Chapter {
    Chapter {
        start_word,
        title: title.into(),
        summary: format!("About {title}."),
    }
}

#[test]
fn migration_v33_adds_chapters_next_to_the_words() {
    let (db, _dir) = open_test_db();
    let column: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('note_audio_words')
             WHERE name = 'chapters_json'",
            [],
            |r| r.get(0),
        )
        .expect("pragma");
    assert_eq!(column, 1);
}

#[test]
fn chapters_are_stored_and_read_back() {
    let (db, _dir) = open_test_db();
    let audio = transcribed_clip(&db, &["one", "two", "three"]);
    assert_eq!(db.chapters_for_audio(&audio), None);

    let chapters = vec![chapter(0, "Start"), chapter(2, "End")];
    assert!(db.set_audio_chapters(&audio, 3, &chapters).unwrap());
    assert_eq!(db.chapters_for_audio(&audio), Some(chapters));
}

#[test]
fn retranscribing_drops_the_chapters() {
    let (db, _dir) = open_test_db();
    let audio = transcribed_clip(&db, &["one", "two", "three"]);
    db.set_audio_chapters(&audio, 3, &[chapter(0, "Old")])
        .unwrap();

    db.set_audio_transcript(&audio, &transcript(&["new", "words"]))
        .unwrap();
    assert_eq!(db.chapters_for_audio(&audio), None);
}

#[test]
fn chapters_of_a_replaced_transcript_are_not_kept() {
    let (db, _dir) = open_test_db();
    let audio = transcribed_clip(&db, &["one", "two", "three"]);
    db.set_audio_transcript(&audio, &transcript(&["new", "words"]))
        .unwrap();

    let stale = [chapter(0, "Stale")];
    assert!(!db.set_audio_chapters(&audio, 3, &stale).unwrap());
    assert_eq!(db.chapters_for_audio(&audio), None);
}

#[test]
fn chapters_already_there_are_not_overwritten() {
    let (db, _dir) = open_test_db();
    let audio = transcribed_clip(&db, &["one", "two"]);
    db.set_audio_chapters(&audio, 2, &[chapter(0, "First")])
        .unwrap();

    assert!(!db
        .set_audio_chapters(&audio, 2, &[chapter(0, "Second")])
        .unwrap());
    assert_eq!(
        db.chapters_for_audio(&audio),
        Some(vec![chapter(0, "First")])
    );
}

// Three paragraphs: words 0..4, 4..9 and 9..12.
const PARAGRAPHS: [std::ops::Range<usize>; 3] = [0..4, 4..9, 9..12];

#[test]
fn a_chapter_starts_on_the_first_word_of_its_paragraph() {
    let reply = "```json\n{\"chapters\":[\
        {\"start\":1,\"title\":\" Welcome \",\"summary\":\"Goals.\"},\
        {\"start\":3,\"title\":\"Budget\",\"summary\":\"Signed off.\"}]}\n```";
    let chapters = parse_chapters(reply, &PARAGRAPHS).unwrap();
    assert_eq!(
        chapters,
        vec![
            Chapter {
                start_word: 0,
                title: "Welcome".into(),
                summary: "Goals.".into(),
            },
            Chapter {
                start_word: 9,
                title: "Budget".into(),
                summary: "Signed off.".into(),
            },
        ]
    );
}

#[test]
fn a_misplaced_chapter_is_dropped() {
    let reply = r#"{"chapters":[
        {"start":2,"title":"Kept","summary":""},
        {"start":1,"title":"Goes back","summary":""},
        {"start":2,"title":"Same start","summary":""},
        {"start":0,"title":"Before the first","summary":""},
        {"start":4,"title":"Past the end","summary":""},
        {"start":3,"title":"  ","summary":""}]}"#;
    let chapters = parse_chapters(reply, &PARAGRAPHS).unwrap();
    let titles: Vec<_> = chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Kept"]);
}

#[test]
fn an_answer_without_chapters_is_an_error() {
    assert!(parse_chapters("Sorry, I cannot help.", &PARAGRAPHS).is_err());
    assert!(parse_chapters(r#"{"chapters":[]}"#, &PARAGRAPHS).is_err());
}
