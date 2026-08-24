// Pull a space by cursor and apply the delta (proposal 0002 T09/T10).
//
// The delta is the ONLY way space content reaches this device, deletions
// included: a tombstone is a change like any other in the same stream, carried
// by the same cursor. That is what keeps the cost proportional to what changed
// instead of to the size of the corpus.

use super::{client, map_err, SpaceError};
use crate::domain::space::due_for_pull;
use crate::domain::{NewFolder, UpdateFolder, UpdateNote};
use crate::infrastructure::backend::spaces::PullResp;
use crate::infrastructure::persistence::Database;

// My own opaque author handle, learned from the first pulled note flagged
// `own`. The pull payload never states it outright, and it is the same handle
// across every space (it hashes the web identity), so one setting holds it.
const MY_AUTHOR_REF_KEY: &str = "space_author_ref";

/// What one pull changed, for the UI banner and the tests.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PullOutcome {
    pub folders: usize,
    pub notes: usize,
    pub removed: usize,
    /// The membership is gone (revoked, or the space was deleted). The caller
    /// tears the local mirror down; the pull itself never deletes a space.
    pub gone: bool,
}

/// Pull everything changed since the stored cursor, page by page, and apply it.
///
/// The cursor only advances once a page is fully applied: a cursor moved past
/// rows that failed to land would lose them for good, since the server has no
/// way to replay what the client claims to already hold.
pub async fn pull_space(
    db: &Database,
    space_id: &str,
) -> Result<PullOutcome, SpaceError> {
    let c = client(db)?;
    let mut outcome = PullOutcome::default();
    loop {
        let since = db
            .get_space(space_id)
            .map_err(SpaceError::Other)?
            .ok_or(SpaceError::Gone)?
            .cursor;
        let page = match c.pull_space(db, space_id, since).await {
            Ok(p) => p,
            // Revoked, or the space is gone. Left alone, the mirror would sit
            // there forever pretending to be live. Nothing is destroyed: the
            // content becomes ordinary local notes.
            Err(e) if e.is_not_found() => {
                outcome.gone = true;
                super::detach_locally(db, space_id, super::Departure::Revoked);
                return Ok(outcome);
            }
            Err(e) => return Err(map_err(e)),
        };
        let more = page.more;
        let next = page.next_seq;
        let applied = apply_delta(db, space_id, &page);
        outcome.folders += applied.folders;
        outcome.notes += applied.notes;
        outcome.removed += applied.removed;
        db.set_space_cursor(space_id, next)
            .map_err(SpaceError::Other)?;
        // A pull is where deletions land, so it is where the vector purge has
        // to be finished, not merely started.
        crate::application::embed::drain_purges().await;
        if !more {
            return Ok(outcome);
        }
    }
}

/// Pull unless the 30 s floor says it is too soon. This is what the UI calls:
/// opening a space folder twice in a row must not cost two round trips.
pub async fn pull_if_due(
    db: &Database,
    space_id: &str,
) -> Result<PullOutcome, SpaceError> {
    let space = db
        .get_space(space_id)
        .map_err(SpaceError::Other)?
        .ok_or(SpaceError::Gone)?;
    if !due_for_pull(space.last_pull_at.as_deref(), chrono::Utc::now()) {
        return Ok(PullOutcome::default());
    }
    pull_space(db, space_id).await
}

/// Refresh every joined space that is due. Runs when the app comes to the
/// foreground; a device with no space does no work at all.
pub async fn pull_all_due(db: &Database) -> usize {
    let mut changed = 0usize;
    for space in db.list_spaces().unwrap_or_default() {
        if !due_for_pull(space.last_pull_at.as_deref(), chrono::Utc::now()) {
            continue;
        }
        match pull_space(db, &space.id).await {
            Ok(o) => changed += o.folders + o.notes + o.removed,
            // offline, revoked, frozen: nothing to do here, the screens that
            // care report it. A failed refresh never blocks the app.
            Err(_) => continue,
        }
    }
    changed
}

/// Apply one page of delta to the local mirror.
///
/// Folders land in two passes: create and rename first, reparent second. A
/// child can arrive in the same page as a parent it does not yet resolve, and
/// hanging it at the root "for now" would show the user a tree that is briefly
/// wrong.
pub fn apply_delta(
    db: &Database,
    space_id: &str,
    page: &PullResp,
) -> PullOutcome {
    let mut outcome = PullOutcome::default();

    for f in &page.folders {
        let local = db.local_folder_for_remote(space_id, &f.id);
        match (f.deleted, local) {
            (true, Some(local_id)) => {
                let _ = db.delete_folder(&local_id);
                outcome.folders += 1;
            }
            (true, None) => {}
            (false, Some(local_id)) => {
                let _ = db.update_folder(
                    &local_id,
                    &UpdateFolder {
                        name: Some(f.name.clone()),
                        description: None,
                        parent_id: None,
                    },
                );
                let _ = db
                    .mark_folder_in_space(&local_id, space_id, &f.id, &f.mode);
                outcome.folders += 1;
            }
            (false, None) => {
                let Ok(created) = db.create_folder(&NewFolder {
                    name: f.name.clone(),
                    description: None,
                    parent_id: None,
                }) else {
                    continue;
                };
                let _ = db.mark_folder_in_space(
                    &created.id,
                    space_id,
                    &f.id,
                    &f.mode,
                );
                outcome.folders += 1;
            }
        }
    }

    for f in page.folders.iter().filter(|f| !f.deleted) {
        let Some(local_id) = db.local_folder_for_remote(space_id, &f.id) else {
            continue;
        };
        let parent_local = f
            .parent_id
            .as_deref()
            .and_then(|p| db.local_folder_for_remote(space_id, p));
        let _ = db.update_folder(
            &local_id,
            &UpdateFolder {
                name: None,
                description: None,
                parent_id: Some(parent_local),
            },
        );
    }

    for n in &page.notes {
        if n.own {
            if let Some(r) = n.author_ref.as_deref() {
                remember_my_author_ref(db, r);
            }
        }
        let local = db.local_note_for_remote(space_id, &n.id);
        match (n.deleted, local) {
            (true, Some(local_id)) => {
                // the full local delete: audio, chunks, share/provenance rows,
                // then the vector purge (queued so a LanceDB failure retries)
                crate::application::note_persistence::delete_note(
                    db, &local_id,
                );
                outcome.removed += 1;
            }
            (true, None) => {}
            (false, local) => {
                let Some(content) = n.content.clone() else {
                    continue;
                };
                let title = n.title.clone().unwrap_or_default();
                let folder_local = n
                    .folder_id
                    .as_deref()
                    .and_then(|f| db.local_folder_for_remote(space_id, f));
                let local_id = match local {
                    Some(id) => {
                        // the server copy wins outright. A local edit by a
                        // non-author was never legitimate, and merging would
                        // reopen the authority question the design closes.
                        let _ = db.update_note(
                            &id,
                            &UpdateNote {
                                title: Some(title.clone()),
                                content: Some(content.clone()),
                                tags: None,
                            },
                        );
                        relink_folder(db, &id, folder_local.as_deref());
                        id
                    }
                    None => {
                        let Some((created, _)) =
                            crate::application::note_persistence::create_note(
                                db,
                                &title,
                                &content,
                                vec![],
                                folder_local.as_deref(),
                                None,
                            )
                        else {
                            continue;
                        };
                        created.id
                    }
                };
                let _ = db.mark_note_in_space(
                    &local_id,
                    space_id,
                    &n.id,
                    n.author_ref.as_deref(),
                );
                // An ordinary note from here on: same embed pipeline, so it is
                // searchable and usable in chat with no code of its own.
                crate::application::embed::embed_note(
                    local_id,
                    title,
                    content,
                    vec![],
                    n.updated_at.clone(),
                );
                outcome.notes += 1;
            }
        }
    }

    outcome
}

fn relink_folder(db: &Database, note_id: &str, folder_id: Option<&str>) {
    let current: Vec<String> = db
        .folders_for_note(note_id)
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.id)
        .collect();
    let unchanged = match folder_id {
        Some(f) => current.len() == 1 && current[0] == f,
        None => current.is_empty(),
    };
    if unchanged {
        return;
    }
    for old in &current {
        let _ = db.remove_note_from_folder(note_id, old);
    }
    if let Some(f) = folder_id {
        let _ = db.add_note_to_folder(note_id, f);
    }
}

fn remember_my_author_ref(db: &Database, author_ref: &str) {
    if db.get_setting(MY_AUTHOR_REF_KEY).as_deref() != Some(author_ref) {
        let _ = db.set_setting(MY_AUTHOR_REF_KEY, author_ref);
    }
}

/// My own author handle, once a pull has taught it. None on a device that has
/// never pulled a note of its own.
pub fn my_author_ref(db: &Database) -> Option<String> {
    db.get_setting(MY_AUTHOR_REF_KEY)
}
