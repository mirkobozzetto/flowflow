use flowflow::domain::{NewTextNote, Transcript, Word};
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::{peers, protocol};
use std::net::TcpListener;
use std::sync::{Arc, Once};
use tempfile::tempdir;

static ADVERTISE: Once = Once::new();

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

fn sync_once(
    db_host: &Arc<Database>,
    db_client: &Arc<Database>,
    host_device_id: &str,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 50, None)
    });
    protocol::sync_with_peer(db_client, host_device_id, "127.0.0.1", port, 50)
        .expect("client sync");
    server.join().expect("join").expect("server sync");
}

/// R6, the hazard the whole storage design was shaped around.
///
/// The sync applier upserts a `note_audio` with `INSERT OR REPLACE`, which deletes
/// the conflicting row and inserts a fresh one - so any column outside the wire
/// spec is reset. Word timings live on their own table precisely so an echo from
/// a peer cannot wipe them on the device that actually holds the audio file.
///
/// If someone later "simplifies" the sidecar into a `words_json` column on
/// `note_audios`, this test is what fails.
#[test]
fn a_remote_note_audio_update_does_not_wipe_local_word_timings() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);

    let note = db_a
        .create_text_note(&NewTextNote {
            title: Some("dictée".into()),
            content: String::new(),
            tags: vec![],
        })
        .expect("create note");
    let audio = db_a
        .add_audio(&note.id, "clip.wav", 9.0)
        .expect("add audio");

    let transcript = Transcript::new(vec![
        Word::new("Bonjour", 0, 500, 0.98),
        Word::new("le", 500, 700, 0.91),
        Word::new("monde", 700, 1200, 0.95),
    ]);
    db_a.set_audio_transcript(&audio.id, &transcript)
        .expect("write transcript on A");

    sync_once(&db_a, &db_b, &id_a);

    // B holds the row (never the audio file), edits the text, and pushes it back.
    let on_b = db_b.list_audios(&note.id).expect("list on B");
    assert_eq!(on_b.len(), 1, "the audio row reached B");
    db_b.set_audio_transcript(
        &audio.id,
        &Transcript::from_text("Bonjour le monde entier"),
    )
    .expect("edit on B");

    sync_once(&db_a, &db_b, &id_a);

    let words_on_a = db_a
        .words_for_audio(&audio.id)
        .expect("A still holds its word timings after the echo");
    assert_eq!(words_on_a, transcript);

    let text_on_a = db_a.list_audios(&note.id).expect("list on A")[0]
        .transcription
        .clone()
        .expect("text");
    assert_eq!(
        text_on_a, "Bonjour le monde entier",
        "the text is synced and did converge; only the words stayed local"
    );
}

/// The other half of the contract: word timings are device-local, so B - which has
/// no audio file to seek into - never receives them.
#[test]
fn word_timings_never_travel_to_the_peer() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);

    let note = db_a
        .create_text_note(&NewTextNote {
            title: Some("dictée".into()),
            content: String::new(),
            tags: vec![],
        })
        .expect("create note");
    let audio = db_a
        .add_audio(&note.id, "clip.wav", 9.0)
        .expect("add audio");
    db_a.set_audio_transcript(
        &audio.id,
        &Transcript::new(vec![Word::new("Bonjour", 0, 500, 0.98)]),
    )
    .expect("write transcript on A");

    sync_once(&db_a, &db_b, &id_a);

    assert!(
        db_b.words_for_audio(&audio.id).is_none(),
        "a peer without the audio file has nothing to seek into"
    );
}
