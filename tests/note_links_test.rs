// note_links repo: V16 migration, recompute upsert semantics (dismissed never
// comes back, pinned survives), backlinks, labels, state transitions.

use flowflow::domain::NewTextNote;
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

fn note(db: &Database, title: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some(title.to_string()),
        content: format!("contenu de {title}"),
        tags: vec![],
    })
    .expect("create note")
    .id
}

#[test]
fn replace_stores_active_links_with_scores() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    let c = note(&db, "C");
    db.replace_note_links(&a, &[(b.clone(), 0.9), (c.clone(), 0.5)])
        .unwrap();

    let links = db.note_links_for(&a).unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].other_note_id, b);
    assert_eq!(links[0].title, "B");
    assert!(links[0].label.is_none());
    assert!(db.has_note_links(&a).unwrap());
    assert!(!db.has_note_links(&b).unwrap());
}

#[test]
fn dismissed_link_never_comes_back() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    db.replace_note_links(&a, &[(b.clone(), 0.9)]).unwrap();
    db.set_note_link_pair_state(&a, &b, "dismissed").unwrap();

    // Recompute still finds b as a candidate: the row must stay dismissed.
    db.replace_note_links(&a, &[(b.clone(), 0.95)]).unwrap();
    assert!(db.note_links_for(&a).unwrap().is_empty());
    // Any-state rows exist -> UI must NOT fall back to live search.
    assert!(db.has_note_links(&a).unwrap());
    // Hidden in both directions.
    assert!(db.note_backlinks_for(&b).unwrap().is_empty());
}

#[test]
fn pinned_link_survives_recompute_and_ranks_first() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    let c = note(&db, "C");
    db.replace_note_links(&a, &[(b.clone(), 0.9), (c.clone(), 0.5)])
        .unwrap();
    db.set_note_link_pair_state(&a, &c, "pinned").unwrap();

    // c drops out of the new candidate set but stays, pinned first.
    db.replace_note_links(&a, &[(b.clone(), 0.9)]).unwrap();
    let links = db.note_links_for(&a).unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].other_note_id, c);
    assert!(links[0].pinned);
    assert_eq!(links[1].other_note_id, b);
}

#[test]
fn backlinks_read_the_reverse_direction() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    db.replace_note_links(&a, &[(b.clone(), 0.7)]).unwrap();

    let back = db.note_backlinks_for(&b).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].other_note_id, a);
    assert_eq!(back[0].title, "A");
}

#[test]
fn labels_fill_once_and_clear_from_pending() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    db.replace_note_links(&a, &[(b.clone(), 0.7)]).unwrap();

    assert_eq!(db.unlabeled_note_links(&a).unwrap(), vec![b.clone()]);
    db.set_note_link_label(&a, &b, "aussi à propos de Jean")
        .unwrap();
    assert!(db.unlabeled_note_links(&a).unwrap().is_empty());
    let links = db.note_links_for(&a).unwrap();
    assert_eq!(links[0].label.as_deref(), Some("aussi à propos de Jean"));

    // Recompute keeps the existing label (only the score refreshes).
    db.replace_note_links(&a, &[(b.clone(), 0.8)]).unwrap();
    let links = db.note_links_for(&a).unwrap();
    assert_eq!(links[0].label.as_deref(), Some("aussi à propos de Jean"));
    assert!((links[0].score - 0.8).abs() < 1e-9);
}

#[test]
fn invalid_state_and_unknown_dst_are_rejected() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    db.replace_note_links(&a, &[(b.clone(), 0.7)]).unwrap();
    assert!(db.set_note_link_pair_state(&a, &b, "weird").is_err());

    // A candidate pointing at a deleted/unknown note is skipped, not an error.
    db.replace_note_links(&a, &[("ghost".to_string(), 0.9)])
        .unwrap();
    assert!(db.note_links_for(&a).unwrap().is_empty());
}

#[test]
fn pair_state_applies_to_both_directions() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    // Each note computed its own row for the same pair.
    db.replace_note_links(&a, &[(b.clone(), 0.9)]).unwrap();
    db.replace_note_links(&b, &[(a.clone(), 0.9)]).unwrap();

    // Dismissing from either side must silence BOTH rows, or the merged
    // list resurfaces the link through the surviving direction.
    db.set_note_link_pair_state(&a, &b, "dismissed").unwrap();
    assert!(db.note_links_for(&a).unwrap().is_empty());
    assert!(db.note_links_for(&b).unwrap().is_empty());
    assert!(db.note_backlinks_for(&a).unwrap().is_empty());
    assert!(db.note_backlinks_for(&b).unwrap().is_empty());
}

#[test]
fn deleting_a_note_cascades_its_links() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    db.replace_note_links(&a, &[(b.clone(), 0.7)]).unwrap();
    db.delete_note(&b).unwrap();
    assert!(db.note_links_for(&a).unwrap().is_empty());
    assert!(!db.has_note_links(&a).unwrap());
}

#[test]
fn empty_replace_clears_active_only() {
    let (db, _dir) = open_test_db();
    let a = note(&db, "A");
    let b = note(&db, "B");
    let c = note(&db, "C");
    db.replace_note_links(&a, &[(b.clone(), 0.9), (c.clone(), 0.5)])
        .unwrap();
    db.set_note_link_pair_state(&a, &b, "pinned").unwrap();

    // Note shrank below embed size: active links purge, pinned stays.
    db.replace_note_links(&a, &[]).unwrap();
    let links = db.note_links_for(&a).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].other_note_id, b);
}
