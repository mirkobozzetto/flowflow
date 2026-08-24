// Share an EXISTING local theme, subtree and notes included (proposal 0002
// listed this as a non-goal; it is the natural gesture, so it lives here as a
// deliberate extension).
//
// The whole point is that the user does not rebuild their tree by hand: the
// theme they already have becomes the space. Every local row is then MARKED
// with its remote id, so the next pull recognises it instead of creating a
// duplicate of everything.
//
// One-way on purpose: this hands the content to a server-authoritative plane.
// Nothing here deletes or moves a local row, so a failure halfway leaves a
// partially-shared tree that a retry finishes rather than corrupts.

use super::{client, map_err, pull::pull_space, SpaceError};
use crate::domain::space::MODE_COLLAB;
use crate::domain::{flatten_tree, subtree_ids};
use crate::infrastructure::persistence::Database;
use std::collections::{HashMap, HashSet};

/// Turn a local theme into a shared space. Returns the space id.
///
/// The space takes the theme's own name. Every folder in the subtree is
/// created collaborative: the owner can lock any of them afterwards, and
/// starting locked would make an invited member unable to do anything.
pub async fn share_existing_folder(
    db: &Database,
    local_folder_id: &str,
) -> Result<String, SpaceError> {
    let root = db
        .get_folder(local_folder_id)
        .map_err(SpaceError::Other)?
        .ok_or_else(|| SpaceError::Other("theme not found".into()))?;
    if root.space_id.is_some() {
        return Err(SpaceError::Other("theme already shared".into()));
    }

    let all = db.list_all_folders().map_err(SpaceError::Other)?;
    let in_subtree: HashSet<String> =
        subtree_ids(&all, local_folder_id).into_iter().collect();
    // flatten_tree is parent-before-child, which is exactly the order the
    // server needs: a folder cannot be created under a parent it has not seen.
    let ordered: Vec<_> = flatten_tree(&all)
        .into_iter()
        .map(|(f, _)| f)
        .filter(|f| in_subtree.contains(&f.id))
        .collect();

    let space_id = super::create_space(db, &root.name).await?;
    let c = client(db)?;
    let mut remote_of: HashMap<String, String> = HashMap::new();

    for folder in &ordered {
        // the root of the shared subtree hangs at the space root, whatever its
        // local parent was
        let parent_remote = if folder.id == local_folder_id {
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
                &space_id,
                None,
                parent_remote.as_deref(),
                &folder.name,
                MODE_COLLAB,
            )
            .await
            .map_err(map_err)?;
        db.mark_folder_in_space(&folder.id, &space_id, &resp.id, MODE_COLLAB)
            .map_err(SpaceError::Other)?;
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
            if note.content.trim().is_empty() || !pushed.insert(note.id.clone())
            {
                continue;
            }
            let resp = c
                .put_space_note(
                    db,
                    &space_id,
                    None,
                    Some(folder_remote),
                    note.title.as_deref(),
                    &note.content,
                )
                .await
                .map_err(map_err)?;
            db.mark_note_in_space(
                &note.id,
                &space_id,
                &resp.id,
                note.author_ref.as_deref(),
            )
            .map_err(SpaceError::Other)?;
        }
    }

    pull_space(db, &space_id).await?;
    Ok(space_id)
}
