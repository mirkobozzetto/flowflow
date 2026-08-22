use crate::domain::Note;
use crate::infrastructure::persistence::Database;

const AUTHOR_ID_PREFIX_LEN: usize = 8;

/// Display label for a note's author, only when it was written by ANOTHER
/// device: the peer's name if known, else the first 8 chars of its UUID.
/// Local notes (and pre-V23 rows without an author) return None, so a
/// single-device user never sees the feature.
pub fn author_label(db: &Database, note: &Note) -> Option<String> {
    let author = note.author_device.as_deref()?;
    let local = db.get_setting("sync_device_id")?;
    if author == local {
        return None;
    }
    let peer_name = db
        .get_peer(author)
        .ok()
        .flatten()
        .and_then(|p| p.name)
        .filter(|n| !n.is_empty());
    Some(
        peer_name.unwrap_or_else(|| {
            author.chars().take(AUTHOR_ID_PREFIX_LEN).collect()
        }),
    )
}

/// Same label for surfaces that only hold a note id (chat source cards).
pub fn author_label_for_note_id(
    db: &Database,
    note_id: &str,
) -> Option<String> {
    let note = db.get_note(note_id).ok().flatten()?;
    author_label(db, &note)
}
