// Share an EXISTING local theme, subtree and notes included, into a space.
// A space is a team: it is named by the user, never after a theme, and takes
// as many themes as the owner drops in.
//
// The whole point is that the user does not rebuild their tree by hand. Every
// local row is MARKED with its remote id, so the next pull recognises it
// instead of creating a duplicate of everything.
//
// One-way on purpose: this hands the content to a server-authoritative plane.
// Nothing here deletes a local row, so a failure halfway leaves a partially
// shared tree that the next pull finishes rather than corrupts.

use super::{client, map_err, pull::pull_space, SpaceError};
use crate::domain::space::{MODE_COLLAB, MODE_READ};
use crate::domain::{flatten_tree, subtree_ids, UpdateFolder};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::persistence::Database;
use std::collections::{HashMap, HashSet};

/// Where a theme goes: a team the user already owns, or a new one.
#[derive(Debug, Clone, PartialEq)]
pub enum ShareTarget {
    Existing(String),
    New(String),
}

/// Share a local theme into a space. Returns the space id. `collaborative`
/// applies to the whole subtree; the owner can flip any folder afterwards.
pub async fn share_existing_folder(
    db: &Database,
    local_folder_id: &str,
    target: ShareTarget,
    collaborative: bool,
) -> Result<String, SpaceError> {
    let root = db
        .get_folder(local_folder_id)
        .map_err(SpaceError::Other)?
        .ok_or_else(|| SpaceError::Other("theme not found".into()))?;
    // a theme already carrying its space is a run that died halfway: finish
    // it there, never in a second space
    let space_id = match root.space_id.clone() {
        Some(existing) => existing,
        None => match target {
            ShareTarget::Existing(id) => id,
            ShareTarget::New(name) => super::create_space(db, &name).await?,
        },
    };
    let mode = if collaborative {
        MODE_COLLAB
    } else {
        MODE_READ
    };
    let c = client(db)?;
    push_subtree(db, &c, &space_id, local_folder_id, mode).await?;
    pull_space(db, &space_id).await?;
    Ok(space_id)
}

/// Finish every share of this space that died halfway: push the rows of its
/// themes that carry no remote id yet. Runs at the head of a pull; a failure
/// waits for the next one.
pub async fn resume_adoptions(db: &Database, space_id: &str) {
    let Ok(c) = client(db) else { return };
    let roots = db.list_root_folders().unwrap_or_default();
    for root in roots
        .iter()
        .filter(|f| f.space_id.as_deref() == Some(space_id))
    {
        let mode = root.mode.as_deref().unwrap_or(MODE_COLLAB);
        if let Err(e) = push_subtree(db, &c, space_id, &root.id, mode).await {
            eprintln!("[space] resume share {}: {e}", root.id);
        }
    }
}

// Push what is not marked yet under `local_root`, parent before child. The
// root hangs at the space root whatever its local parent was: a nested theme
// left under its old parent would never show under its space section.
async fn push_subtree(
    db: &Database,
    c: &BackendClient,
    space_id: &str,
    local_root: &str,
    mode: &str,
) -> Result<(), SpaceError> {
    let all = db.list_all_folders().map_err(SpaceError::Other)?;
    let in_subtree: HashSet<String> =
        subtree_ids(&all, local_root).into_iter().collect();
    let ordered: Vec<_> = flatten_tree(&all)
        .into_iter()
        .map(|(f, _)| f)
        .filter(|f| in_subtree.contains(&f.id))
        .collect();

    let mut remote_of: HashMap<String, String> = ordered
        .iter()
        .filter_map(|f| Some((f.id.clone(), f.remote_id.clone()?)))
        .collect();

    for folder in &ordered {
        if remote_of.contains_key(&folder.id) {
            continue;
        }
        let is_root = folder.id == local_root;
        let parent_remote = if is_root {
            None
        } else {
            folder
                .parent_id
                .as_deref()
                .and_then(|p| remote_of.get(p))
                .cloned()
        };
        let resp = c
            .put_space_folder(
                db,
                space_id,
                None,
                parent_remote.as_deref(),
                &folder.name,
                mode,
            )
            .await
            .map_err(map_err)?;
        db.mark_folder_in_space(&folder.id, space_id, &resp.id, mode)
            .map_err(SpaceError::Other)?;
        if is_root && folder.parent_id.is_some() {
            db.update_folder(
                &folder.id,
                &UpdateFolder {
                    name: None,
                    description: None,
                    parent_id: Some(None),
                },
            )
            .map_err(SpaceError::Other)?;
        }
        remote_of.insert(folder.id.clone(), resp.id);
    }

    // A note can sit in two themes of the same subtree; push it once, into the
    // first one that claims it.
    let mut pushed: HashSet<String> = HashSet::new();
    for folder in &ordered {
        let Some(folder_remote) = remote_of.get(&folder.id) else {
            continue;
        };
        for note in db.list_notes_in_folder(&folder.id).unwrap_or_default() {
            if note.remote_id.is_some()
                || note.content.trim().is_empty()
                || !pushed.insert(note.id.clone())
            {
                continue;
            }
            let resp = c
                .put_space_note(
                    db,
                    space_id,
                    None,
                    Some(folder_remote),
                    note.title.as_deref(),
                    &note.content,
                )
                .await
                .map_err(map_err)?;
            db.mark_note_in_space(
                &note.id,
                space_id,
                &resp.id,
                note.author_ref.as_deref(),
            )
            .map_err(SpaceError::Other)?;
        }
    }
    Ok(())
}
