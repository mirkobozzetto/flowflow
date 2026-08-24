// Pure tests for the chat "@" menu search: matching over title/tags/content,
// scoped-first partition, untitled excerpt labels, cap.

use flowflow::application::mention::search_mentions;
use flowflow::domain::{Note, NoteType};
use std::collections::HashSet;

fn note(id: &str, title: Option<&str>, content: &str, tags: &[&str]) -> Note {
    Note {
        id: id.into(),
        note_type: NoteType::Text,
        title: title.map(String::from),
        content: content.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        sources_json: None,
        thread_id: None,
        author_device: None,
        space_id: None,
        remote_id: None,
        author_ref: None,
        created_at: format!("2026-06-0{}T10:00:00Z", id.len().min(9)),
        modified_at: "2026-06-01T10:00:00Z".into(),
    }
}

#[test]
fn empty_query_returns_notes_in_incoming_order_capped() {
    let notes: Vec<Note> = (0..12)
        .map(|i| note(&format!("n{i}"), Some(&format!("Titre {i}")), "x", &[]))
        .collect();
    let res = search_mentions(&notes, "", None, 8);
    assert!(res.in_scope.is_empty());
    assert_eq!(res.others.len(), 8);
    assert_eq!(res.others[0].note_id, "n0");
}

#[test]
fn matches_title_tags_and_content() {
    let notes = vec![
        note("a", Some("Réunion budget"), "rien", &[]),
        note("b", Some("Divers"), "on a parlé du budget avec Jean", &[]),
        note("c", Some("Courses"), "lait", &["budget"]),
        note("d", Some("Vacances"), "plage", &["été"]),
    ];
    let res = search_mentions(&notes, "budget", None, 8);
    let ids: Vec<&str> =
        res.others.iter().map(|h| h.note_id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}

#[test]
fn match_is_case_insensitive_and_substring() {
    let notes = vec![note("a", Some("Réunion Budget"), "", &[])];
    let res = search_mentions(&notes, "budg", None, 8);
    assert_eq!(res.others.len(), 1);
}

#[test]
fn scoped_notes_come_first_and_keep_their_slots() {
    let mut notes: Vec<Note> = (0..10)
        .map(|i| note(&format!("out{i}"), Some(&format!("Hors {i}")), "x", &[]))
        .collect();
    notes.push(note("in1", Some("Dedans"), "x", &[]));
    let scoped: HashSet<String> = ["in1".to_string()].into_iter().collect();
    let res = search_mentions(&notes, "", Some(&scoped), 8);
    assert_eq!(res.in_scope.len(), 1);
    assert_eq!(res.in_scope[0].note_id, "in1");
    assert_eq!(res.others.len(), 7);
}

#[test]
fn untitled_note_gets_content_excerpt_not_dropped() {
    let notes = vec![note(
        "u",
        None,
        "| a | b |\nidée de pitch pour la startup",
        &[],
    )];
    let res = search_mentions(&notes, "pitch", None, 8);
    assert_eq!(res.others.len(), 1);
    let hit = &res.others[0];
    assert!(hit.untitled);
    assert!(hit.label.contains("pitch") || hit.label.contains("idée"));
    assert!(!hit.label.contains('|'));
}

#[test]
fn hit_date_is_day_prefix() {
    let notes = vec![note("ab", Some("T"), "x", &[])];
    let res = search_mentions(&notes, "", None, 8);
    assert_eq!(res.others[0].date, "2026-06-02");
}

#[test]
fn truly_empty_note_is_skipped() {
    let notes = vec![note("e", None, "   ", &[])];
    let res = search_mentions(&notes, "", None, 8);
    assert!(res.others.is_empty());
}
