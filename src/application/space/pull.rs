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
use crate::infrastructure::backend::BackendError;
use crate::infrastructure::persistence::{now_iso, Database, DbTx};

// My own opaque author handle, learned from the first pulled note flagged
// `own`. The pull payload never states it outright, and it is the same handle
// across every space (it hashes the web identity), so one setting holds it.
const MY_AUTHOR_REF_KEY: &str = "space_author_ref";

// How many staged notes one pull pushes before reading: enough to drain a
// day offline, small enough that a stuck one never delays the pull for long.
const REPUBLISH_CAP: usize = 20;

/// What one pull changed, for the UI banner and the tests.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PullOutcome {
    pub folders: usize,
    pub notes: usize,
    pub removed: usize,
    pub threads: usize,
    /// The membership is gone (revoked, or the space was deleted). The caller
    /// tears the local mirror down; the pull itself never deletes a space.
    pub gone: bool,
}

/// What a committed page still owes outside SQLite. Files and vectors do not
/// roll back, so they wait for the commit; a crash in between leaves orphans
/// the replayable purge and the next embed pick up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageEffects {
    /// (local note id, audio files) of every note a tombstone removed
    pub removed_notes: Vec<(String, Vec<String>)>,
    /// (local note id, title, content, updated_at) of every note applied
    pub embed: Vec<(String, String, String, String)>,
}

/// Pull everything changed since the stored cursor, page by page, and apply it.
///
/// A page and its cursor commit together: a cursor moved past rows that failed
/// to land would lose them for good, since the server has no way to replay
/// what the client claims to already hold. A page that fails stops the pull,
/// cursor intact, and is replayed next time.
pub async fn pull_space(
    db: &Database,
    space_id: &str,
) -> Result<PullOutcome, SpaceError> {
    let c = client(db)?;
    super::resume_adoptions(db, space_id).await;
    republish_pending(db, space_id).await;
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
            // Too soon after the last pull: the server's freshness floor, not a
            // failure. The next scheduled pull reads what this one could not.
            Err(BackendError::Status(429, _)) => return Ok(outcome),
            Err(e) => return Err(map_err(e)),
        };
        let more = page.more;
        let (applied, effects) = db
            .apply_space_page(space_id, page.next_seq, |tx| {
                apply_delta(tx, space_id, &page)
            })
            .map_err(SpaceError::Other)?;
        outcome.folders += applied.folders;
        outcome.notes += applied.notes;
        outcome.removed += applied.removed;
        outcome.threads += applied.threads;
        run_effects(db, effects).await;
        if !more {
            return Ok(outcome);
        }
    }
}

async fn run_effects(db: &Database, effects: PageEffects) {
    for (id, audio_paths) in &effects.removed_notes {
        crate::application::note_persistence::finish_note_delete(
            db,
            id,
            audio_paths,
        );
    }
    for (id, title, content, updated_at) in effects.embed {
        // An ordinary note from here on: same embed pipeline, so it is
        // searchable and usable in chat with no code of its own.
        crate::application::embed::embed_note(
            id,
            title,
            content,
            vec![],
            updated_at,
        );
    }
    // A pull is where deletions land, so it is where the vector purge has to
    // be finished, not merely started.
    crate::application::embed::drain_purges().await;
}

/// Push the notes saved into this space that the server has not confirmed
/// yet, at most `REPUBLISH_CAP` of them, those whose retry time has come.
/// Whatever happens to them, the pull goes on. Returns how many were tried.
pub async fn republish_pending(db: &Database, space_id: &str) -> usize {
    let due = db
        .due_note_publishes(space_id, &now_iso(), REPUBLISH_CAP)
        .unwrap_or_default();
    for note_id in &due {
        if let Err(e) = super::write::publish_local_note(db, note_id).await {
            eprintln!("[space] republish {note_id}: {e}");
        }
    }
    due.len()
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

/// Claim the spaces another device of this account already joined.
///
/// `spaces` is device-local while a folder's `space_id` travels over P2P
/// sync, so a paired device can hold a space's whole tree with no row to list
/// it under: the sidebar then shows the folder nowhere. Membership is per
/// account, so the server tells this device whether it belongs; if it does,
/// the row is created at cursor 0 and the next pull replays the space onto
/// rows already keyed by remote id, without duplicates. Any error leaves
/// the folder as it is: a later session retries.
pub async fn adopt_synced_spaces(db: &Database) -> usize {
    let Ok(c) = client(db) else { return 0 };
    let mut adopted = 0usize;
    for (space_id, name) in db.unknown_spaces().unwrap_or_default() {
        let Ok(members) = c.list_space_members(db, &space_id).await else {
            continue;
        };
        let Some(me) = members.iter().find(|m| m.me) else {
            continue;
        };
        if db.upsert_space(&space_id, &name, me.is_owner).is_ok() {
            adopted += 1;
        }
    }
    adopted
}

/// Refresh every joined space that is due, after claiming the spaces that
/// reached this device through sync. Runs when the app comes to the
/// foreground; a device with no space does one query and no round trip.
pub async fn pull_all_due(db: &Database) -> usize {
    let mut changed = adopt_synced_spaces(db).await;
    for space in db.list_spaces().unwrap_or_default() {
        if !due_for_pull(space.last_pull_at.as_deref(), chrono::Utc::now()) {
            continue;
        }
        match pull_space(db, &space.id).await {
            Ok(o) => changed += o.folders + o.notes + o.removed + o.threads,
            // offline, revoked, frozen: nothing to do here, the screens that
            // care report it. A failed refresh never blocks the app.
            Err(_) => continue,
        }
    }
    changed
}

/// Apply one page of delta to the local mirror, inside the caller's
/// transaction. Any SQL failure is returned, not skipped: a row the server
/// will never replay must not be dropped on the floor. A tombstone for a row
/// this device never had is a no-op.
///
/// Folders land in two passes: create and rename first, reparent second. A
/// child can arrive in the same page as a parent it does not yet resolve, and
/// hanging it at the root "for now" would show the user a tree that is briefly
/// wrong.
pub fn apply_delta(
    tx: &DbTx,
    space_id: &str,
    page: &PullResp,
) -> Result<(PullOutcome, PageEffects), String> {
    let mut outcome = PullOutcome::default();
    let mut effects = PageEffects::default();

    for f in &page.folders {
        let local = tx.local_folder_for_remote(space_id, &f.id);
        match (f.deleted, local) {
            (true, Some(local_id)) => {
                tx.delete_folder(&local_id)?;
                outcome.folders += 1;
            }
            (true, None) => {}
            (false, Some(local_id)) => {
                tx.update_folder(
                    &local_id,
                    &UpdateFolder {
                        name: Some(f.name.clone()),
                        description: None,
                        parent_id: None,
                    },
                )?;
                tx.mark_folder_in_space(&local_id, space_id, &f.id, &f.mode)?;
                outcome.folders += 1;
            }
            (false, None) => {
                let created = tx.create_folder(&NewFolder {
                    name: f.name.clone(),
                    description: None,
                    parent_id: None,
                })?;
                tx.mark_folder_in_space(&created.id, space_id, &f.id, &f.mode)?;
                outcome.folders += 1;
            }
        }
    }

    for f in page.folders.iter().filter(|f| !f.deleted) {
        let Some(local_id) = tx.local_folder_for_remote(space_id, &f.id) else {
            continue;
        };
        let parent_local = f
            .parent_id
            .as_deref()
            .and_then(|p| tx.local_folder_for_remote(space_id, p));
        tx.update_folder(
            &local_id,
            &UpdateFolder {
                name: None,
                description: None,
                parent_id: Some(parent_local),
            },
        )?;
    }

    // Threads share ids with the server, so a page carries them without a
    // mapping. Members arrive as ordinary note rows with a thread_id.
    for t in &page.threads {
        if t.deleted {
            if tx.thread_exists(&t.id)? {
                tx.delete_thread(&t.id)?;
                outcome.threads += 1;
            }
            continue;
        }
        let folder_local = t
            .folder_id
            .as_deref()
            .and_then(|f| tx.local_folder_for_remote(space_id, f));
        tx.upsert_thread_with_id(&t.id, &t.title, folder_local.as_deref())?;
        outcome.threads += 1;
    }

    for n in &page.notes {
        if n.own {
            if let Some(r) = n.author_ref.as_deref() {
                remember_my_author_ref(tx, r)?;
            }
        }
        let local = tx.local_note_for_remote(space_id, &n.id);
        match (n.deleted, local) {
            (true, Some(local_id)) => {
                let audio =
                    crate::application::note_persistence::delete_note_rows(
                        tx, &local_id,
                    )?;
                effects.removed_notes.push((local_id, audio));
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
                    .and_then(|f| tx.local_folder_for_remote(space_id, f));
                let local_id = match local {
                    Some(id) => {
                        // the server copy wins outright. A local edit by a
                        // non-author was never legitimate, and merging would
                        // reopen the authority question the design closes.
                        tx.update_note(
                            &id,
                            &UpdateNote {
                                title: Some(title.clone()),
                                content: Some(content.clone()),
                                tags: None,
                            },
                        )?;
                        relink_folder(tx, &id, folder_local.as_deref())?;
                        id
                    }
                    None => {
                        crate::application::note_persistence::create_note_in(
                            tx,
                            &title,
                            &content,
                            vec![],
                            folder_local.as_deref(),
                            None,
                        )?
                        .0
                        .id
                    }
                };
                tx.mark_note_in_space(
                    &local_id,
                    space_id,
                    &n.id,
                    n.author_ref.as_deref(),
                )?;
                // a thread this device has not mirrored yet leaves the note
                // unthreaded until the thread row lands; the server replays it
                let thread_local = match n.thread_id.as_deref() {
                    Some(id) if tx.thread_exists(id)? => Some(id),
                    _ => None,
                };
                tx.set_note_thread(&local_id, thread_local)?;
                effects.embed.push((
                    local_id,
                    title,
                    content,
                    n.updated_at.clone(),
                ));
                outcome.notes += 1;
            }
        }
    }

    Ok((outcome, effects))
}

fn relink_folder(
    tx: &DbTx,
    note_id: &str,
    folder_id: Option<&str>,
) -> Result<(), String> {
    let current: Vec<String> = tx
        .folders_for_note(note_id)?
        .into_iter()
        .map(|f| f.id)
        .collect();
    let unchanged = match folder_id {
        Some(f) => current.len() == 1 && current[0] == f,
        None => current.is_empty(),
    };
    if unchanged {
        return Ok(());
    }
    for old in &current {
        tx.remove_note_from_folder(note_id, old)?;
    }
    if let Some(f) = folder_id {
        tx.add_note_to_folder(note_id, f)?;
    }
    Ok(())
}

fn remember_my_author_ref(tx: &DbTx, author_ref: &str) -> Result<(), String> {
    if tx.get_setting(MY_AUTHOR_REF_KEY).as_deref() != Some(author_ref) {
        tx.set_setting(MY_AUTHOR_REF_KEY, author_ref)?;
    }
    Ok(())
}

/// My own author handle, once a pull has taught it. None on a device that has
/// never pulled a note of its own.
pub fn my_author_ref(db: &Database) -> Option<String> {
    db.get_setting(MY_AUTHOR_REF_KEY)
}
