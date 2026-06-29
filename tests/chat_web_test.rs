use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

#[test]
fn chat_web_defaults_off_and_round_trips() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("web test").unwrap();
    assert!(
        !db.chat_web(&conv.id),
        "a new chat is off the web by default"
    );

    db.set_chat_web(&conv.id, true).unwrap();
    assert!(db.chat_web(&conv.id), "toggle on must persist");

    db.set_chat_web(&conv.id, false).unwrap();
    assert!(!db.chat_web(&conv.id), "toggle off must persist");
}

#[test]
fn chat_web_key_is_removed_with_conversation() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("web cleanup").unwrap();
    db.set_chat_web(&conv.id, true).unwrap();

    db.delete_conversation(&conv.id).unwrap();

    assert!(!db.chat_web(&conv.id));
    assert_eq!(
        db.get_setting(&format!("chat_web:{}", conv.id)),
        None,
        "web flag key must be removed with the conversation"
    );
}
