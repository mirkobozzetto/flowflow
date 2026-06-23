use flowflow::domain::ChatScope;
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

#[test]
fn chat_scope_persists_and_clears_with_conversation() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("scope test").unwrap();
    assert_eq!(db.chat_scope(&conv.id), None);

    let folder1 = ChatScope::Folder("folder-1".to_string());
    db.set_chat_scope(&conv.id, Some(&folder1)).unwrap();
    assert_eq!(db.chat_scope(&conv.id), Some(folder1));

    db.set_chat_scope(&conv.id, None).unwrap();
    assert_eq!(db.chat_scope(&conv.id), None);

    db.set_chat_scope(
        &conv.id,
        Some(&ChatScope::Folder("folder-2".to_string())),
    )
    .unwrap();
    db.delete_conversation(&conv.id).unwrap();
    assert_eq!(db.chat_scope(&conv.id), None);
    assert_eq!(
        db.get_setting(&format!("chat_scope:{}", conv.id)),
        None,
        "scope key must be removed with the conversation"
    );
}

#[test]
fn chat_scope_tags_thread_and_folder_and_legacy() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");

    let conv = db.create_conversation("tag test").unwrap();

    let thread = ChatScope::Thread("thr-1".to_string());
    db.set_chat_scope(&conv.id, Some(&thread)).unwrap();
    assert_eq!(db.chat_scope(&conv.id), Some(thread));
    assert_eq!(
        db.get_setting(&format!("chat_scope:{}", conv.id)),
        Some("thread:thr-1".to_string()),
        "thread scope must persist tagged"
    );

    let folder = ChatScope::Folder("fld-1".to_string());
    db.set_chat_scope(&conv.id, Some(&folder)).unwrap();
    assert_eq!(
        db.get_setting(&format!("chat_scope:{}", conv.id)),
        Some("folder:fld-1".to_string())
    );

    db.set_setting(&format!("chat_scope:{}", conv.id), "legacy-folder")
        .unwrap();
    assert_eq!(
        db.chat_scope(&conv.id),
        Some(ChatScope::Folder("legacy-folder".to_string())),
        "legacy bare id must read as a folder scope"
    );
}
