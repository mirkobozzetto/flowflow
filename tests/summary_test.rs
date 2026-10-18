use flowflow::application::note_persistence::delete_note;
use flowflow::application::summary::{numbered_body, parse_summary};
use flowflow::domain::summary::{source_hash, Moment, Summary, SummaryItem};
use flowflow::domain::{NewTextNote, Word};
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::{peers, protocol};
use std::net::TcpListener;
use std::sync::{Arc, Once};
use tempfile::tempdir;

const LANG: &str = "fr";
const MODEL: &str = "claude-haiku-5-5";
const BODY: &str = "We ship on Friday. Sarah books the room.";

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

fn sync_once(db_host: &Arc<Database>, db_client: &Arc<Database>, host: &str) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 50, None)
    });
    protocol::sync_with_peer(db_client, host, "127.0.0.1", port, 50)
        .expect("client sync");
    server.join().expect("join").expect("server sync");
}

fn note(db: &Database) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("meeting".into()),
        content: BODY.into(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn summary() -> Summary {
    Summary {
        key_points: vec![SummaryItem {
            text: "Release on Friday.".into(),
            at: None,
        }],
        actions: vec![SummaryItem {
            text: "Sarah: book the room.".into(),
            at: Some(Moment {
                audio_id: "clip-1".into(),
                ms: 724_000,
            }),
        }],
        ..Summary::default()
    }
}

#[test]
fn migration_v34_creates_the_summaries_table() {
    let (db, _dir) = open_db();
    let columns: Vec<String> = db
        .conn()
        .prepare("SELECT name FROM pragma_table_info('note_summaries')")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .flatten()
        .collect();
    for col in ["id", "body_json", "lang", "model", "source_hash"] {
        assert!(columns.contains(&col.to_string()), "{col} missing");
    }
}

#[test]
fn a_summary_is_stored_read_back_and_replaced() {
    let (db, _dir) = open_db();
    let id = note(&db);
    let hash = source_hash(BODY);
    db.save_summary(&id, &summary(), LANG, MODEL, &hash)
        .expect("save");

    let kept = db.summary_for_note(&id).expect("summary");
    assert_eq!(kept.body, summary());
    assert_eq!(kept.source_hash, hash);
    assert_eq!(kept.model, MODEL);

    let redone = Summary {
        decisions: vec![SummaryItem {
            text: "No workshop on Monday.".into(),
            at: None,
        }],
        ..Summary::default()
    };
    db.save_summary(&id, &redone, LANG, MODEL, &hash)
        .expect("redo");
    assert_eq!(db.summary_for_note(&id).expect("summary").body, redone);
}

#[test]
fn the_fingerprint_changes_with_the_text_only() {
    assert_eq!(source_hash(BODY), source_hash(&format!("\n{BODY}  \n")));
    assert_ne!(source_hash(BODY), source_hash("We ship on Monday."));
}

#[test]
fn deleting_the_note_deletes_its_summary() {
    let (db, _dir) = open_db();
    let id = note(&db);
    db.save_summary(&id, &summary(), LANG, MODEL, &source_hash(BODY))
        .expect("save");
    delete_note(&db, &id).expect("delete note");
    assert!(db.summary_for_note(&id).is_none());
}

#[test]
fn a_summary_and_its_removal_reach_the_other_device() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let id_a = pair(&db_a, &db_b);
    let id = note(&db_a);
    db_a.save_summary(&id, &summary(), LANG, MODEL, &source_hash(BODY))
        .expect("save on A");

    sync_once(&db_a, &db_b, &id_a);
    let on_b = db_b.summary_for_note(&id).expect("summary reached B");
    assert_eq!(on_b.body, summary());
    assert_eq!(on_b.source_hash, source_hash(BODY));

    db_b.delete_summary(&id).expect("delete on B");
    sync_once(&db_a, &db_b, &id_a);
    assert!(db_a.summary_for_note(&id).is_none(), "removal reached A");
}

fn words(text: &str, from_ms: u32) -> Vec<Word> {
    text.split(' ')
        .enumerate()
        .map(|(i, w)| {
            let at = from_ms + i as u32 * 400;
            Word::new(w, at, at + 300, 0.9)
        })
        .collect()
}

#[test]
fn the_note_goes_numbered_with_where_each_paragraph_starts() {
    let mut spoken = words("we ship on friday", 0);
    spoken.extend(words("sarah books the room", 64_000));
    let clips = vec![("clip-1".to_string(), spoken)];
    let content = "we ship on friday\n\nSarah books the big room";
    let (input, moments) = numbered_body(content, &clips);
    assert_eq!(
        input,
        "[1] 0:00 we ship on friday\nSarah books the big room"
    );
    assert_eq!(
        moments,
        vec![Moment {
            audio_id: "clip-1".into(),
            ms: 0
        }]
    );
}

#[test]
fn a_cited_paragraph_gives_the_item_its_moment() {
    let moments = vec![
        Moment {
            audio_id: "clip-1".into(),
            ms: 0,
        },
        Moment {
            audio_id: "clip-1".into(),
            ms: 724_000,
        },
    ];
    let reply =
        "```json\n{\"key_points\":[{\"text\":\" Release on Friday. \"}],\
        \"actions\":[{\"text\":\"Sarah: book the room.\",\"at\":2},\
        {\"text\":\"Tom: call the venue.\",\"at\":9}],\"questions\":[]}\n```";
    let summary = parse_summary(reply, &moments).expect("summary");
    assert_eq!(summary.key_points[0].text, "Release on Friday.");
    assert_eq!(summary.actions[0].at, Some(moments[1].clone()));
    assert_eq!(summary.actions[1].at, None, "no paragraph 9");
    assert!(summary.decisions.is_empty());
}

#[test]
fn an_item_written_as_a_bare_sentence_is_kept_without_a_time() {
    let reply = "{\"decisions\":[\"Delete the archived repo.\", 42],\
        \"actions\":[{\"text\":\"Tom: call the venue.\"}]}";
    let summary = parse_summary(reply, &[]).expect("summary");
    assert_eq!(summary.decisions.len(), 1, "the number is dropped alone");
    assert_eq!(summary.decisions[0].text, "Delete the archived repo.");
    assert_eq!(summary.decisions[0].at, None);
    assert_eq!(summary.actions.len(), 1);
}

#[test]
fn an_action_split_in_who_and_what_with_a_written_number_keeps_its_time() {
    let moments = vec![Moment {
        audio_id: "clip-1".into(),
        ms: 61_000,
    }];
    let reply = "{\"actions\":[{\"who\":\"Sarah\",\"what\":\"book the room\",\
        \"at\":\"[1]\"}]}";
    let summary = parse_summary(reply, &moments).expect("summary");
    assert_eq!(summary.actions[0].text, "Sarah: book the room");
    assert_eq!(summary.actions[0].at, Some(moments[0].clone()));
}

#[test]
fn an_answer_without_items_is_an_error() {
    assert!(parse_summary("{\"key_points\":[{\"text\":\" \"}]}", &[]).is_err());
    assert!(parse_summary("not json", &[]).is_err());
}
