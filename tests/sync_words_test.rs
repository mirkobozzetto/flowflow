use flowflow::application::chapters::transcribed_here;
use flowflow::domain::reading::Chapter;
use flowflow::domain::{NewTextNote, Transcript, Word};
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::{peers, protocol};
use std::net::TcpListener;
use std::sync::{Arc, Once};
use tempfile::tempdir;

static ADVERTISE: Once = Once::new();

const BATCH_ROWS: usize = 50;
const OLD_PROTOCOL: i64 = 6;
const WORDS_KIND: &str = "note_audio_words";
const CHAPTERS_KIND: &str = "note_audio_chapters";

fn open_db() -> (Arc<Database>, tempfile::TempDir) {
    let dir = tempdir().expect("db tempdir");
    let db =
        Database::open_at(dir.path().join("flowflow.db")).expect("open_at");
    (Arc::new(db), dir)
}

fn pair(db_a: &Arc<Database>, db_b: &Arc<Database>) -> String {
    ADVERTISE.call_once(|| {
        std::env::set_var("FLOWFLOW_SYNC_ADVERTISE_ADDR", "127.0.0.1");
    });
    let host = peers::start_pairing_host(db_a.clone()).expect("host");
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    let uri = peers::encode_pairing_uri(&payload).expect("re-encode uri");
    let id_a = peers::join_pairing(db_b, &uri).expect("join pairing");
    for _ in 0..100 {
        if host.status() != peers::PairingStatus::Waiting {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    id_a
}

fn try_sync(
    db_host: &Arc<Database>,
    db_client: &Arc<Database>,
    host_device_id: &str,
    batch_rows: usize,
    client_version: i64,
) -> Result<protocol::SyncStats, String> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, batch_rows, None)
    });
    let client = protocol::sync_with_peer_speaking(
        db_client,
        host_device_id,
        "127.0.0.1",
        port,
        batch_rows,
        client_version,
    );
    let served = server.join().expect("join");
    match (client, served) {
        (Ok(stats), Ok(_)) => Ok(stats),
        (client, served) => Err(format!(
            "client: {:?} / server: {:?}",
            client.err().map(|e| e.to_string()),
            served.err().map(|e| e.to_string())
        )),
    }
}

fn sync_once(
    db_host: &Arc<Database>,
    db_client: &Arc<Database>,
    host_device_id: &str,
) {
    try_sync(
        db_host,
        db_client,
        host_device_id,
        BATCH_ROWS,
        protocol::PROTOCOL_VERSION,
    )
    .expect("sync");
}

fn new_note(db: &Database) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("réunion".into()),
        content: String::new(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn clip(db: &Database) -> (String, String) {
    let note = new_note(db);
    let audio = db.add_audio(&note, "clip.wav", 9.0).expect("add audio");
    (note, audio.id)
}

fn words(texts: &[&str]) -> Transcript {
    Transcript::new(
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let at = i as u32 * 500;
                Word::new(*t, at, at + 400, 0.9)
            })
            .collect(),
    )
}

fn chapters() -> Vec<Chapter> {
    vec![
        Chapter {
            start_word: 0,
            title: "Ouverture".into(),
            summary: "On se dit bonjour.".into(),
        },
        Chapter {
            start_word: 2,
            title: "Budget".into(),
            summary: "Le budget est validé.".into(),
        },
    ]
}

fn transcribed_with_chapters(db: &Database) -> (String, String) {
    let (note, audio) = clip(db);
    db.set_audio_transcript(&audio, &words(&["Bonjour", "à", "tous"]))
        .expect("transcript");
    assert!(db.set_audio_chapters(&audio, 3, &chapters()).expect("set"));
    (note, audio)
}

fn rows_of(db: &Database, table: &str, id: &str) -> i64 {
    db.conn()
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1"),
            [id],
            |r| r.get(0),
        )
        .expect("count")
}

fn conflicts_of(db: &Database, kind: &str) -> i64 {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE entity_kind = ?1",
            [kind],
            |r| r.get(0),
        )
        .expect("conflicts")
}

fn cursors(db: &Database) -> (i64, Option<String>) {
    let acked: i64 = db
        .conn()
        .query_row("SELECT last_acked_seq FROM sync_peers", [], |r| r.get(0))
        .expect("watermark");
    let acked_by = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key LIKE 'sync_peer_acked_by_%'",
            [],
            |r| r.get(0),
        )
        .ok();
    (acked, acked_by)
}

/// AC1: a meeting transcribed on one device reads the same on the other.
#[test]
fn words_and_chapters_reach_the_client() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = transcribed_with_chapters(&db_a);

    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(db_b.words_for_audio(&audio), db_a.words_for_audio(&audio));
    assert_eq!(db_b.chapters_for_audio(&audio), Some(chapters()));
}

/// AC2: the other way round, the client's meeting reaches the host.
#[test]
fn words_and_chapters_reach_the_host() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = transcribed_with_chapters(&db_b);

    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(db_a.words_for_audio(&audio), db_b.words_for_audio(&audio));
    assert_eq!(db_a.chapters_for_audio(&audio), Some(chapters()));
}

/// AC5: a retranscription replaces the words everywhere and drops the old
/// chapters everywhere.
#[test]
fn a_retranscription_replaces_words_and_drops_chapters_on_the_peer() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = transcribed_with_chapters(&db_a);
    sync_once(&db_a, &db_b, &id_a);

    let fresh = words(&["Nouvelle", "version"]);
    db_a.set_audio_transcript(&audio, &fresh).expect("retranscribe");
    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(db_b.words_for_audio(&audio), Some(fresh));
    assert_eq!(db_b.chapters_for_audio(&audio), None);
}

/// AC6: deleting a clip removes its words and chapters on the peer.
#[test]
fn a_deleted_clip_leaves_no_words_on_the_peer() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = transcribed_with_chapters(&db_a);
    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(rows_of(&db_b, WORDS_KIND, &audio), 1);

    db_a.delete_audio(&audio).expect("delete clip");
    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(rows_of(&db_b, WORDS_KIND, &audio), 0);
    assert_eq!(rows_of(&db_b, CHAPTERS_KIND, &audio), 0);
}

/// AC6: deleting the whole note does the same.
#[test]
fn a_deleted_note_leaves_no_words_on_the_peer() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (note, audio) = transcribed_with_chapters(&db_a);
    sync_once(&db_a, &db_b, &id_a);

    db_a.delete_note(&note).expect("delete note");
    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(rows_of(&db_b, WORDS_KIND, &audio), 0);
    assert_eq!(rows_of(&db_b, CHAPTERS_KIND, &audio), 0);
}

/// AC6: "delete all recordings" goes through the same path.
#[test]
fn deleting_all_recordings_leaves_no_words_on_the_peer() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = transcribed_with_chapters(&db_a);
    sync_once(&db_a, &db_b, &id_a);

    let audio_dir = tempdir().expect("audio dir");
    db_a.delete_all_audios(audio_dir.path().to_str().unwrap())
        .expect("delete all");
    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(rows_of(&db_b, WORDS_KIND, &audio), 0);
    assert_eq!(rows_of(&db_b, CHAPTERS_KIND, &audio), 0);
}

/// AC6: "delete my data" also removes the words and chapters received.
#[test]
fn wiping_local_content_removes_received_words_and_chapters() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = transcribed_with_chapters(&db_a);
    sync_once(&db_a, &db_b, &id_a);

    db_b.wipe_local_content().expect("wipe");

    assert_eq!(rows_of(&db_b, WORDS_KIND, &audio), 0);
    assert_eq!(rows_of(&db_b, CHAPTERS_KIND, &audio), 0);
}

/// AC7 (R6): the applier upserts a `note_audio` with `INSERT OR REPLACE`. Word
/// timings live on their own table precisely so an echo of the clip row never
/// wipes them, on either side.
#[test]
fn a_remote_note_audio_update_does_not_wipe_word_timings() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (note, audio) = transcribed_with_chapters(&db_a);
    let transcript = db_a.words_for_audio(&audio).expect("words on A");
    sync_once(&db_a, &db_b, &id_a);

    db_b.set_audio_transcription(&audio, "Bonjour à tous, corrigé")
        .expect("edit the clip text on B");
    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(db_a.words_for_audio(&audio), Some(transcript.clone()));
    assert_eq!(db_b.words_for_audio(&audio), Some(transcript));
    assert_eq!(db_a.chapters_for_audio(&audio), Some(chapters()));
    let text_on_a = db_a.list_audios(&note).expect("list on A")[0]
        .transcription
        .clone();
    assert_eq!(text_on_a.as_deref(), Some("Bonjour à tous, corrigé"));
}

/// AC9: a device still on the old protocol is refused, and nothing moves.
#[test]
fn an_old_protocol_peer_is_refused_without_consuming_anything() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    transcribed_with_chapters(&db_a);
    transcribed_with_chapters(&db_b);
    let before = (cursors(&db_a), cursors(&db_b));

    let refused = try_sync(&db_a, &db_b, &id_a, BATCH_ROWS, OLD_PROTOCOL)
        .expect_err("a v6 peer must be refused");

    assert!(refused.contains("update the app on both devices"), "{refused}");
    assert_eq!((cursors(&db_a), cursors(&db_b)), before);
}

/// AC4: only the device that transcribed a clip makes its chapters.
#[test]
fn only_the_transcribing_device_makes_the_chapters() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = clip(&db_a);
    db_a.set_audio_transcript(&audio, &words(&["Bonjour", "à", "tous"]))
        .expect("transcript");
    sync_once(&db_a, &db_b, &id_a);

    assert!(transcribed_here(&db_a, &audio));
    assert!(!transcribed_here(&db_b, &audio));
}

/// AC4: two concurrent sets of chapters settle on one, with no conflict entry.
#[test]
fn concurrent_chapters_converge_without_a_conflict() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, audio) = clip(&db_a);
    db_a.set_audio_transcript(&audio, &words(&["Bonjour", "à", "tous"]))
        .expect("transcript");
    sync_once(&db_a, &db_b, &id_a);

    let mut other = chapters();
    other[1].title = "Finances".into();
    assert!(db_a.set_audio_chapters(&audio, 3, &chapters()).unwrap());
    assert!(db_b.set_audio_chapters(&audio, 3, &other).unwrap());
    sync_once(&db_a, &db_b, &id_a);
    sync_once(&db_a, &db_b, &id_a);

    let on_a = db_a.chapters_for_audio(&audio).expect("chapters on A");
    assert_eq!(db_b.chapters_for_audio(&audio), Some(on_a));
    assert_eq!(conflicts_of(&db_a, CHAPTERS_KIND), 0);
    assert_eq!(conflicts_of(&db_b, CHAPTERS_KIND), 0);
}

fn checkpoint_and_copy(
    db: &Database,
    from: &std::path::Path,
    to: &std::path::Path,
) {
    db.checkpoint_wal().expect("checkpoint");
    std::fs::copy(from, to).expect("backup copy");
}

/// AC4, full-state branch: words this side never tracked (written before v7)
/// arrive in a session where it is the authority. They are kept, not archived
/// as a conflict and re-deleted on the device that transcribed them.
#[test]
fn untracked_words_in_a_full_state_session_are_kept_on_both_sides() {
    let (db_a, _da) = open_db();
    let (db_b, dir_b) = open_db();
    let id_a = pair(&db_a, &db_b);
    let path_b = dir_b.path().join("flowflow.db");
    let backup_b = dir_b.path().join("backup.db");

    let (_, audio) = clip(&db_a);
    sync_once(&db_a, &db_b, &id_a);
    db_b.set_audio_transcript(&audio, &words(&["Bonjour", "à", "tous"]))
        .expect("transcribe on B");
    checkpoint_and_copy(&db_b, &path_b, &backup_b);
    new_note(&db_b);
    sync_once(&db_a, &db_b, &id_a);

    // A forgets the words as if it had run v6: no row, no meta.
    db_a.conn()
        .execute_batch(&format!(
            "DELETE FROM {WORDS_KIND};
             DELETE FROM sync_row_meta WHERE entity_kind = '{WORDS_KIND}';"
        ))
        .expect("forget words on A");

    db_b.checkpoint_wal().expect("checkpoint before restore");
    drop(db_b);
    let _ = std::fs::remove_file(dir_b.path().join("flowflow.db-wal"));
    let _ = std::fs::remove_file(dir_b.path().join("flowflow.db-shm"));
    std::fs::copy(&backup_b, &path_b).expect("restore copy");
    let db_b = Arc::new(Database::open_at(path_b).expect("reopen restored"));

    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(rows_of(&db_a, WORDS_KIND, &audio), 1);
    assert_eq!(rows_of(&db_b, WORDS_KIND, &audio), 1);
    assert_eq!(conflicts_of(&db_a, WORDS_KIND), 0);
    assert_eq!(conflicts_of(&db_b, WORDS_KIND), 0);
}

// The tables as a v6 build left them: words keyed by audio_id, chapters on
// the same row, nothing tracked.
fn rewind_words_to_v34(db: &Database) {
    db.conn()
        .execute_batch(&format!(
            "DROP TRIGGER IF EXISTS trg_sync_{WORDS_KIND}_ai;
             DROP TRIGGER IF EXISTS trg_sync_{WORDS_KIND}_au;
             DROP TRIGGER IF EXISTS trg_sync_{CHAPTERS_KIND}_ai;
             DROP TRIGGER IF EXISTS trg_sync_{CHAPTERS_KIND}_au;
             DROP TABLE {WORDS_KIND};
             DROP TABLE {CHAPTERS_KIND};
             CREATE TABLE {WORDS_KIND} (
                 audio_id TEXT PRIMARY KEY,
                 words_json TEXT NOT NULL,
                 chapters_json TEXT
             );
             DELETE FROM sync_row_meta
                 WHERE entity_kind IN ('{WORDS_KIND}', '{CHAPTERS_KIND}');
             DELETE FROM _migrations WHERE version >= 35;"
        ))
        .expect("rewind to v34");
}

fn insert_v34_words(db: &Database, audio: &str, with_chapters: bool) {
    let words_json =
        serde_json::to_string(&words(&["Bonjour", "à", "tous"]).words)
            .unwrap();
    let chapters_json =
        with_chapters.then(|| serde_json::to_string(&chapters()).unwrap());
    db.conn()
        .execute(
            &format!(
                "INSERT INTO {WORDS_KIND} (audio_id, words_json, chapters_json)
                 VALUES (?1, ?2, ?3)"
            ),
            rusqlite::params![audio, words_json, chapters_json],
        )
        .expect("insert v34 words");
}

fn reopen(
    db: Arc<Database>,
    dir: &tempfile::TempDir,
) -> Arc<Database> {
    db.checkpoint_wal().expect("checkpoint");
    drop(db);
    Arc::new(
        Database::open_at(dir.path().join("flowflow.db")).expect("reopen"),
    )
}

/// AC3: clips transcribed before v7 travel at the first sync, chapters
/// included; a words row whose clip is gone stays behind.
#[test]
fn words_written_before_v7_travel_at_the_first_sync() {
    let (db_a, dir_a) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let (_, with_chapters) = clip(&db_a);
    let (_, plain) = clip(&db_a);
    sync_once(&db_a, &db_b, &id_a);

    rewind_words_to_v34(&db_a);
    insert_v34_words(&db_a, &with_chapters, true);
    insert_v34_words(&db_a, &plain, false);
    insert_v34_words(&db_a, "orphan-clip", false);
    let db_a = reopen(db_a, &dir_a);

    assert!(transcribed_here(&db_a, &plain));
    sync_once(&db_a, &db_b, &id_a);

    assert_eq!(db_b.words_for_audio(&plain), db_a.words_for_audio(&plain));
    assert_eq!(
        db_b.words_for_audio(&with_chapters),
        db_a.words_for_audio(&with_chapters)
    );
    assert_eq!(db_b.chapters_for_audio(&with_chapters), Some(chapters()));
    assert_eq!(rows_of(&db_b, WORDS_KIND, "orphan-clip"), 0);
}

const MEETINGS: usize = 100;
const MEETING_MS: u32 = 90 * 60 * 1000;
const WORD_EVERY_MS: u32 = 400;

/// AC8: a long history of big meetings goes through, no batch past the
/// transport's message limit. The words rows are consecutive in seq order,
/// as the V35 seeding leaves them.
#[test]
fn a_hundred_long_meetings_sync_in_one_session() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let clips: Vec<String> = (0..MEETINGS).map(|_| clip(&db_a).1).collect();
    let meeting = Transcript::new(
        (0..MEETING_MS / WORD_EVERY_MS)
            .map(|i| {
                let at = i * WORD_EVERY_MS;
                Word::new(format!("mot{i}"), at, at + 300, 0.9)
            })
            .collect(),
    );
    for audio in &clips {
        db_a.set_audio_transcript(audio, &meeting).expect("transcript");
    }

    try_sync(
        &db_a,
        &db_b,
        &id_a,
        protocol::DEFAULT_BATCH_ROWS,
        protocol::PROTOCOL_VERSION,
    )
    .expect("the whole history goes through");

    let arrived: i64 = db_b
        .conn()
        .query_row(&format!("SELECT COUNT(*) FROM {WORDS_KIND}"), [], |r| {
            r.get(0)
        })
        .expect("count");
    assert_eq!(arrived, MEETINGS as i64);
}
