// Pure tree helpers (flatten for indented pickers, subtree ids for the move-folder
// cycle guard) + the recursive folder-scope queries against a real SQLite file.

use flowflow::domain::{
    flatten_tree, subtree_ids, Folder, NewFolder, NewTextNote,
};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn folder(id: &str, name: &str, parent: Option<&str>) -> Folder {
    Folder {
        id: id.into(),
        name: name.into(),
        description: None,
        parent_id: parent.map(String::from),
        created_at: "2026-01-01T00:00:00Z".into(),
        modified_at: "2026-01-01T00:00:00Z".into(),
    }
}

#[test]
fn flatten_orders_roots_then_children_with_depth() {
    let folders = vec![
        folder("b", "Bravo", None),
        folder("a", "alpha", None),
        folder("b1", "Sub", Some("b")),
        folder("b1a", "Deep", Some("b1")),
    ];
    let flat = flatten_tree(&folders);
    let names: Vec<(String, u32)> =
        flat.into_iter().map(|(f, d)| (f.name, d)).collect();
    assert_eq!(
        names,
        vec![
            ("alpha".to_string(), 0),
            ("Bravo".to_string(), 0),
            ("Sub".to_string(), 1),
            ("Deep".to_string(), 2),
        ]
    );
}

#[test]
fn flatten_appends_orphans_at_root() {
    let folders =
        vec![folder("a", "A", None), folder("lost", "Lost", Some("gone"))];
    let flat = flatten_tree(&folders);
    assert_eq!(flat.len(), 2);
    assert!(flat.iter().any(|(f, d)| f.id == "lost" && *d == 0));
}

#[test]
fn subtree_ids_covers_self_and_descendants_only() {
    let folders = vec![
        folder("a", "A", None),
        folder("b", "B", Some("a")),
        folder("c", "C", Some("b")),
        folder("x", "X", None),
    ];
    let mut ids = subtree_ids(&folders, "a");
    ids.sort();
    assert_eq!(ids, vec!["a", "b", "c"]);
    assert_eq!(subtree_ids(&folders, "x"), vec!["x"]);
}

#[test]
fn folder_tree_queries_include_subfolders() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("test.db")).expect("open");

    let parent = db
        .create_folder(&NewFolder {
            name: "Travail".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    let child = db
        .create_folder(&NewFolder {
            name: "Clients".into(),
            description: None,
            parent_id: Some(parent.id.clone()),
        })
        .unwrap();

    let in_parent = db
        .create_text_note(&NewTextNote {
            title: Some("racine".into()),
            content: "note racine".into(),
            tags: vec![],
        })
        .unwrap();
    let in_child = db
        .create_text_note(&NewTextNote {
            title: Some("feuille".into()),
            content: "note feuille".into(),
            tags: vec![],
        })
        .unwrap();
    db.add_note_to_folder(&in_parent.id, &parent.id).unwrap();
    db.add_note_to_folder(&in_child.id, &child.id).unwrap();

    let direct = db.list_notes_in_folder(&parent.id).unwrap();
    assert_eq!(direct.len(), 1);

    let tree = db.list_notes_in_folder_tree(&parent.id).unwrap();
    let mut ids: Vec<String> = tree.into_iter().map(|n| n.id).collect();
    ids.sort();
    let mut expected = vec![in_parent.id.clone(), in_child.id.clone()];
    expected.sort();
    assert_eq!(ids, expected);

    assert_eq!(db.count_notes_in_folder_tree(&parent.id).unwrap(), 2);
    assert_eq!(db.count_notes_in_folder_tree(&child.id).unwrap(), 1);
}

#[test]
fn folder_tree_count_deduplicates_note_in_two_folders_of_same_tree() {
    let dir = tempdir().expect("dir");
    let db = Database::open_at(dir.path().join("test.db")).expect("open");

    let parent = db
        .create_folder(&NewFolder {
            name: "P".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();
    let child = db
        .create_folder(&NewFolder {
            name: "C".into(),
            description: None,
            parent_id: Some(parent.id.clone()),
        })
        .unwrap();
    let note = db
        .create_text_note(&NewTextNote {
            title: Some("double".into()),
            content: "n".into(),
            tags: vec![],
        })
        .unwrap();
    db.add_note_to_folder(&note.id, &parent.id).unwrap();
    db.add_note_to_folder(&note.id, &child.id).unwrap();

    assert_eq!(db.count_notes_in_folder_tree(&parent.id).unwrap(), 1);
    assert_eq!(db.list_notes_in_folder_tree(&parent.id).unwrap().len(), 1);
}
