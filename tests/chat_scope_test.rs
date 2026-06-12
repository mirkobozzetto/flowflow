use flowflow::db::Database;
use tempfile::tempdir;

#[test]
fn chat_scope_persists_and_clears_with_conversation() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("scope test").unwrap();
    assert_eq!(db.chat_scope(&conv.id), None);

    db.set_chat_scope(&conv.id, Some("folder-1")).unwrap();
    assert_eq!(db.chat_scope(&conv.id), Some("folder-1".to_string()));

    db.set_chat_scope(&conv.id, None).unwrap();
    assert_eq!(db.chat_scope(&conv.id), None);

    db.set_chat_scope(&conv.id, Some("folder-2")).unwrap();
    db.delete_conversation(&conv.id).unwrap();
    assert_eq!(db.chat_scope(&conv.id), None);
    assert_eq!(
        db.get_setting(&format!("chat_scope:{}", conv.id)),
        None,
        "scope key must be removed with the conversation"
    );
}
