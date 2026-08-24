// Use cases for shared notes/threads (proposal 0001): publish/republish,
// revoke, open a code, append/edit/delete one's own notes, keep a note with
// frozen provenance, and the deletion-alignment pass that keeps every phone
// honest with the backend (lifecycle rules 1-5).

use crate::domain::share::{
    LocalShare, Provenance, ShareKind, PROVENANCE_GONE, PROVENANCE_LIVE,
};
use crate::infrastructure::backend::shares::{
    PublishNote, RemoteSharedNote, RemoteThread,
};
use crate::infrastructure::backend::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(Debug, Clone, PartialEq)]
pub enum ShareError {
    // no backend configured: the feature is dark
    NoBackend,
    // 403: premium or linked web account missing (the backend does not say which)
    Refused,
    // the uniform dead-code 404 (unknown, revoked, expired)
    Gone,
    Other(String),
}

impl std::fmt::Display for ShareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShareError::NoBackend => write!(f, "no backend configured"),
            ShareError::Refused => {
                write!(f, "refused (premium + linked account required)")
            }
            ShareError::Gone => write!(f, "share not found"),
            ShareError::Other(e) => write!(f, "{e}"),
        }
    }
}

fn map_err(e: BackendError) -> ShareError {
    match e {
        BackendError::Status(404, _) => ShareError::Gone,
        BackendError::Status(403, _) => ShareError::Refused,
        other => ShareError::Other(other.to_string()),
    }
}

fn client(db: &Database) -> Result<BackendClient, ShareError> {
    BackendClient::from_db(db).ok_or(ShareError::NoBackend)
}

/// Publish (or republish) one note. The previous code for this note dies
/// server-side; the local record is replaced.
pub async fn publish_note(
    db: &Database,
    note_id: &str,
) -> Result<LocalShare, ShareError> {
    let note = db
        .get_note(note_id)
        .ok()
        .flatten()
        .ok_or_else(|| ShareError::Other("note not found".into()))?;
    let title = note.title.clone();
    let notes = vec![PublishNote {
        title: note.title,
        content: note.content,
    }];
    publish(db, &ShareKind::Note, note_id, title.as_deref(), notes).await
}

/// Publish (or republish) a whole app thread as a multi-author fil.
pub async fn publish_thread(
    db: &Database,
    thread_id: &str,
) -> Result<LocalShare, ShareError> {
    let thread = db
        .get_thread(thread_id)
        .ok()
        .flatten()
        .ok_or_else(|| ShareError::Other("thread not found".into()))?;
    let notes: Vec<PublishNote> = db
        .list_thread_notes(thread_id)
        .map_err(ShareError::Other)?
        .into_iter()
        .map(|n| PublishNote {
            title: n.title,
            content: n.content,
        })
        .collect();
    if notes.is_empty() {
        return Err(ShareError::Other("empty thread".into()));
    }
    publish(
        db,
        &ShareKind::Thread,
        thread_id,
        Some(&thread.title),
        notes,
    )
    .await
}

async fn publish(
    db: &Database,
    kind: &ShareKind,
    source_id: &str,
    title: Option<&str>,
    notes: Vec<PublishNote>,
) -> Result<LocalShare, ShareError> {
    let c = client(db)?;
    let resp = c
        .publish_share(db, kind.as_str(), source_id, title, &notes)
        .await
        .map_err(map_err)?;
    db.upsert_share(source_id, kind, &resp.code, &resp.expires_at)
        .map_err(ShareError::Other)?;
    Ok(LocalShare {
        source_id: source_id.to_string(),
        kind: kind.clone(),
        code: resp.code,
        expires_at: resp.expires_at,
        created_at: String::new(),
    })
}

/// Revoke my share for a source. The backend 404 (already dead) still clears
/// the local record: the outcome the user asked for is "no live link".
pub async fn revoke(db: &Database, source_id: &str) -> Result<(), ShareError> {
    let Some(share) = db.get_share(source_id) else {
        return Ok(());
    };
    let c = client(db)?;
    match c.revoke_share(db, &share.code).await {
        Ok(()) => {}
        Err(e) if e.is_not_found() => {}
        Err(e) => return Err(map_err(e)),
    }
    db.delete_share(source_id).map_err(ShareError::Other)
}

/// Open a code (read-only view / shared-thread screen).
pub async fn open(
    db: &Database,
    code: &str,
) -> Result<RemoteThread, ShareError> {
    let c = client(db)?;
    c.read_share(db, code).await.map_err(map_err)
}

/// Append my own note to a shared fil. Returns the remote note id.
pub async fn append(
    db: &Database,
    code: &str,
    title: Option<&str>,
    content: &str,
) -> Result<String, ShareError> {
    let c = client(db)?;
    c.append_share_note(db, code, title, content)
        .await
        .map_err(map_err)
}

/// Edit my own note in a shared fil.
pub async fn update_own(
    db: &Database,
    code: &str,
    remote_note_id: &str,
    title: Option<&str>,
    content: &str,
) -> Result<(), ShareError> {
    let c = client(db)?;
    c.update_share_note(db, code, remote_note_id, title, content)
        .await
        .map_err(map_err)
}

/// Tombstone my own note in a shared fil (the deletion signal).
pub async fn delete_own(
    db: &Database,
    code: &str,
    remote_note_id: &str,
) -> Result<(), ShareError> {
    let c = client(db)?;
    c.delete_share_note(db, code, remote_note_id)
        .await
        .map_err(map_err)
}

/// Report a share (authenticated + deduplicated server-side; never auto-hides).
pub async fn report(db: &Database, code: &str) -> Result<(), ShareError> {
    let c = client(db)?;
    c.report_share(db, code).await.map_err(map_err)
}

/// Tombstone every shared note I authored, everywhere. Lifecycle rule 5:
/// run this BEFORE leaving/deleting the account.
pub async fn delete_my_notes(db: &Database) -> Result<(), ShareError> {
    let c = client(db)?;
    c.delete_my_shared_notes(db).await.map_err(map_err)
}

/// Keep a shared note in my base: a real local note (it re-enters the embed
/// pipeline) plus a FROZEN provenance card. Returns the local note id.
pub fn keep_note(
    db: &Database,
    thread: &RemoteThread,
    note: &RemoteSharedNote,
) -> Result<String, ShareError> {
    let content = note
        .content
        .clone()
        .ok_or_else(|| ShareError::Other("note has no content".into()))?;
    let title = note.title.clone().unwrap_or_default();
    let (created, _) = crate::application::note_persistence::create_note(
        db,
        &title,
        &content,
        vec![],
        None,
        None,
    )
    .ok_or_else(|| ShareError::Other("could not create note".into()))?;
    let author_name = note
        .author
        .display_name
        .clone()
        .or_else(|| thread.owner.display_name.clone());
    db.upsert_provenance(&Provenance {
        note_id: created.id.clone(),
        share_code: thread.code.clone(),
        remote_note_id: note.id.clone(),
        author_name,
        captured_at: String::new(),
        state: PROVENANCE_LIVE.to_string(),
    })
    .map_err(ShareError::Other)?;
    crate::application::embed::embed_note(
        created.id.clone(),
        title,
        content,
        vec![],
        created.created_at.clone(),
    );
    Ok(created.id)
}

// ---- local author block list (App Store 1.2) ----
// CSV of opaque author_refs in settings; blocked authors' notes are hidden on
// this device. Never synced to the backend: blocking is a reader-side choice.

const BLOCKED_AUTHORS_KEY: &str = "blocked_share_authors";

pub fn blocked_authors(db: &Database) -> Vec<String> {
    db.get_setting(BLOCKED_AUTHORS_KEY)
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn block_author(db: &Database, author_ref: &str) {
    let mut list = blocked_authors(db);
    if !list.iter().any(|r| r == author_ref) {
        list.push(author_ref.to_string());
        let _ = db.set_setting(BLOCKED_AUTHORS_KEY, &list.join(","));
    }
}

pub fn is_blocked(db: &Database, author_ref: Option<&str>) -> bool {
    match author_ref {
        Some(r) => blocked_authors(db).iter().any(|b| b == r),
        None => false,
    }
}

/// One alignment event the UI surfaces as a banner.
#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentEvent {
    // a kept note's author deleted it: the local copy + embeddings are gone
    RemovedByAuthor { note_title: Option<String> },
    // the whole share died (revoked/expired/author erased): provenance greys out
    SourceGone { share_code: String },
}

/// Deletion-alignment pass (lifecycle rules 2-3): re-read every share we hold
/// kept content from. A tombstoned remote note removes the local copy AND its
/// embeddings; a dead code greys the provenance out (the copy stays).
pub async fn align_kept_content(db: &Database) -> Vec<AlignmentEvent> {
    let Ok(c) = client(db) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    for code in db.all_provenance_codes() {
        match c.read_share(db, &code).await {
            Ok(thread) => {
                for p in db.provenances_for_code(&code) {
                    let Some(remote) =
                        thread.notes.iter().find(|n| n.id == p.remote_note_id)
                    else {
                        continue;
                    };
                    if remote.deleted {
                        let title = db
                            .get_note(&p.note_id)
                            .ok()
                            .flatten()
                            .and_then(|n| n.title);
                        crate::application::note_persistence::delete_note(
                            db, &p.note_id,
                        );
                        let _ = db.delete_provenance(&p.note_id);
                        events.push(AlignmentEvent::RemovedByAuthor {
                            note_title: title,
                        });
                    } else if p.state != PROVENANCE_LIVE {
                        let _ = db
                            .set_provenance_state(&p.note_id, PROVENANCE_LIVE);
                    }
                }
            }
            Err(e) if e.is_not_found() => {
                let mut flagged = false;
                for p in db.provenances_for_code(&code) {
                    if p.state != PROVENANCE_GONE {
                        let _ = db
                            .set_provenance_state(&p.note_id, PROVENANCE_GONE);
                        flagged = true;
                    }
                }
                if flagged {
                    events
                        .push(AlignmentEvent::SourceGone { share_code: code });
                }
            }
            // network trouble: change nothing, retry next pass
            Err(_) => {}
        }
    }
    events
}
