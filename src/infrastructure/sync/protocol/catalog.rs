use super::proto_err;
use crate::infrastructure::sync::SyncError;

// Registry of the synced entities (RFC 0004 D8, v1 perimeter). One entry per
// entity_kind tracked in sync_row_meta: which table it lives in, which
// columns travel in the payload, how its entity_id maps to the row key, and
// whether chunk BLOBs ride along. settings, pending_transcriptions and audio
// FILES are deliberately absent (excluded from sync).
pub(super) struct KindSpec {
    pub kind: &'static str,
    pub table: &'static str,
    pub cols: &'static [&'static str],
    pub composite_link: bool,
    pub chunk_owner: bool,
}

pub(super) const KINDS: &[KindSpec] = &[
    KindSpec {
        kind: "note",
        table: "notes",
        cols: &[
            "id",
            "note_type",
            "title",
            "content",
            "tags",
            "sources_json",
            "thread_id",
            "author_device",
            "created_at",
            "modified_at",
        ],
        composite_link: false,
        chunk_owner: true,
    },
    KindSpec {
        kind: "thread",
        table: "threads",
        cols: &["id", "title", "folder_id", "created_at", "modified_at"],
        composite_link: false,
        chunk_owner: false,
    },
    KindSpec {
        kind: "folder",
        table: "folders",
        cols: &[
            "id",
            "name",
            "description",
            "parent_id",
            "created_at",
            "modified_at",
        ],
        composite_link: false,
        chunk_owner: false,
    },
    KindSpec {
        kind: "notes_folders",
        table: "notes_folders",
        cols: &["folder_id", "note_id", "created_at"],
        composite_link: true,
        chunk_owner: false,
    },
    KindSpec {
        kind: "conversation",
        table: "conversations",
        cols: &["id", "title", "created_at", "modified_at"],
        composite_link: false,
        chunk_owner: false,
    },
    KindSpec {
        kind: "conversation_message",
        table: "conversation_messages",
        cols: &[
            "id",
            "conversation_id",
            "role",
            "content",
            "sources_json",
            "created_at",
        ],
        composite_link: false,
        chunk_owner: false,
    },
    KindSpec {
        kind: "attachment",
        table: "attachments",
        cols: &["id", "note_id", "filename", "content_text", "imported_at"],
        composite_link: false,
        chunk_owner: true,
    },
    KindSpec {
        kind: "note_audio",
        table: "note_audios",
        cols: &[
            "id",
            "note_id",
            "file_path",
            "duration_secs",
            "created_at",
            "transcription",
        ],
        composite_link: false,
        chunk_owner: false,
    },
    KindSpec {
        kind: "note_reminder",
        table: "note_reminders",
        cols: &[
            "id",
            "note_id",
            "reminder_id",
            "backend",
            "intent_hash",
            "due_year",
            "due_month",
            "due_day",
            "due_hour",
            "due_minute",
            "is_all_day",
            "tz_id",
            "recurrence",
            "state",
            "created_at",
        ],
        composite_link: false,
        chunk_owner: false,
    },
];

pub(super) fn spec_for(kind: &str) -> Option<&'static KindSpec> {
    KINDS.iter().find(|s| s.kind == kind)
}

// WHERE clause + parameters locating an entity row from its sync entity_id.
// Link rows (notes_folders) encode their composite key as "folder_id:note_id"
// (UUIDs never contain ':'), matching the tracking triggers.
pub(super) fn entity_key_params(
    spec: &KindSpec,
    entity_id: &str,
) -> Result<(String, Vec<String>), SyncError> {
    if spec.composite_link {
        let (folder_id, note_id) =
            entity_id.split_once(':').ok_or_else(|| {
                proto_err(format!("bad link entity id: {entity_id}"))
            })?;
        Ok((
            "folder_id = ?1 AND note_id = ?2".to_string(),
            vec![folder_id.to_string(), note_id.to_string()],
        ))
    } else {
        Ok(("id = ?1".to_string(), vec![entity_id.to_string()]))
    }
}
