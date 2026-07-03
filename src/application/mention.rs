use crate::application::ai::{char_prefix, plain_preview};
use crate::domain::Note;
use std::collections::HashSet;

/// One row of the chat "@" menu, ready to render.
#[derive(Clone, Debug, PartialEq)]
pub struct MentionHit {
    pub note_id: String,
    /// Title when the note has one, else a content excerpt.
    pub label: String,
    pub untitled: bool,
    /// YYYY-MM-DD, same shape the note cards show.
    pub date: String,
}

/// Split of the menu into the chat's scoped notes first, the rest after.
/// `others` holds everything when the chat has no scope.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MentionResults {
    pub in_scope: Vec<MentionHit>,
    pub others: Vec<MentionHit>,
}

fn matches(note: &Note, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let title_hit = note
        .title
        .as_deref()
        .is_some_and(|t| t.to_lowercase().contains(q));
    title_hit
        || note.tags.iter().any(|t| t.to_lowercase().contains(q))
        || note.content.to_lowercase().contains(q)
}

fn to_hit(note: &Note) -> MentionHit {
    let title = note.title.clone().unwrap_or_default();
    let untitled = title.is_empty();
    let label = if untitled {
        char_prefix(plain_preview(&note.content).trim(), 60)
    } else {
        title
    };
    MentionHit {
        note_id: note.id.clone(),
        label,
        untitled,
        date: char_prefix(&note.created_at, 10),
    }
}

/// Filter + rank the "@" menu. Notes arrive newest-first (list_notes order) and
/// stay that way; matching covers title, tags AND content so "the note that talks
/// about X" is reachable even when X never made it into the title. Untitled notes
/// show a content excerpt instead of being hidden. Only mentionable rows count
/// toward `cap`, scoped notes get the slots first.
pub fn search_mentions(
    notes: &[Note],
    query: &str,
    scoped_ids: Option<&HashSet<String>>,
    cap: usize,
) -> MentionResults {
    let q = query.to_lowercase();
    let mut res = MentionResults::default();
    for note in notes {
        if !matches(note, &q) {
            continue;
        }
        let hit = to_hit(note);
        if hit.label.is_empty() {
            continue;
        }
        match scoped_ids {
            Some(ids) if ids.contains(&note.id) => res.in_scope.push(hit),
            _ => res.others.push(hit),
        }
    }
    res.in_scope.truncate(cap);
    res.others.truncate(cap - res.in_scope.len());
    res
}
