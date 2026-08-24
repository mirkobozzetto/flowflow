// Writing into a space (proposal 0002 T09). Every write goes to the server
// first and is mirrored by pulling the row back, so the local copy is always
// the one the server actually stored: no second write path, no divergence to
// reconcile later.
//
// The right to write is checked locally BEFORE the call, against the same rule
// the server applies. That is not a security measure (the server decides), it
// is so the UI can say "you cannot write here" instead of letting the user type
// a note and lose it to a 403.

use super::{client, map_err, pull::pull_space, SpaceError};
use crate::domain::space::{can_write_in, MODE_COLLAB, MODE_READ};
use crate::infrastructure::persistence::Database;

fn guard(
    db: &Database,
    space_id: &str,
    folder_remote_id: Option<&str>,
) -> Result<(), SpaceError> {
    let space = db
        .get_space(space_id)
        .map_err(SpaceError::Other)?
        .ok_or(SpaceError::Gone)?;
    let tree = db.space_folder_tree(space_id).map_err(SpaceError::Other)?;
    if can_write_in(&tree, space.is_owner, folder_remote_id) {
        Ok(())
    } else {
        Err(SpaceError::ReadOnly)
    }
}

/// May the user put content in this folder? The UI asks before showing a
/// compose button, so nobody types into a folder that will refuse them.
pub fn can_write(
    db: &Database,
    space_id: &str,
    folder_remote_id: Option<&str>,
) -> bool {
    guard(db, space_id, folder_remote_id).is_ok()
}

/// Create a folder in the space. `collab` lets other members add notes to it,
/// `read` makes it (and everything under it) read-only for them.
pub async fn create_folder(
    db: &Database,
    space_id: &str,
    parent_remote_id: Option<&str>,
    name: &str,
    collaborative: bool,
) -> Result<String, SpaceError> {
    guard(db, space_id, parent_remote_id)?;
    let mode = if collaborative {
        MODE_COLLAB
    } else {
        MODE_READ
    };
    let c = client(db)?;
    let resp = c
        .put_space_folder(db, space_id, None, parent_remote_id, name, mode)
        .await
        .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(resp.id)
}

/// Rename a folder or change its declared mode. Flipping a parent to `read`
/// restricts its whole subtree at once: no child is rewritten, the ancestor
/// chain simply stops resolving to `collab`.
pub async fn update_folder(
    db: &Database,
    space_id: &str,
    remote_id: &str,
    parent_remote_id: Option<&str>,
    name: &str,
    collaborative: bool,
) -> Result<(), SpaceError> {
    let mode = if collaborative {
        MODE_COLLAB
    } else {
        MODE_READ
    };
    let c = client(db)?;
    c.put_space_folder(
        db,
        space_id,
        Some(remote_id),
        parent_remote_id,
        name,
        mode,
    )
    .await
    .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(())
}

/// Reparent a folder. `None` moves it to the space root. A move under one of
/// its own descendants is refused server-side.
pub async fn move_folder(
    db: &Database,
    space_id: &str,
    remote_id: &str,
    parent_remote_id: Option<&str>,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    c.move_space_folder(db, space_id, remote_id, parent_remote_id)
        .await
        .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(())
}

/// Delete a folder AND everything under it, notes included.
pub async fn delete_folder(
    db: &Database,
    space_id: &str,
    remote_id: &str,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    c.delete_space_folder(db, space_id, remote_id)
        .await
        .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(())
}

/// Put a note in the space. A voice note travels as its TRANSCRIPTION only:
/// the space plane carries text, the audio file stays on the device that
/// recorded it.
pub async fn create_note(
    db: &Database,
    space_id: &str,
    folder_remote_id: Option<&str>,
    title: Option<&str>,
    content: &str,
) -> Result<String, SpaceError> {
    guard(db, space_id, folder_remote_id)?;
    let c = client(db)?;
    let resp = c
        .put_space_note(db, space_id, None, folder_remote_id, title, content)
        .await
        .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(resp.id)
}

/// Edit one's OWN note. A note written by someone else answers the uniform 404:
/// authorship is enforced in SQL server-side, never trusted from here.
pub async fn update_note(
    db: &Database,
    space_id: &str,
    remote_id: &str,
    folder_remote_id: Option<&str>,
    title: Option<&str>,
    content: &str,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    c.put_space_note(
        db,
        space_id,
        Some(remote_id),
        folder_remote_id,
        title,
        content,
    )
    .await
    .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(())
}

/// Tombstone one's OWN note. This is the signal that removes it from every
/// other member's device, index included.
pub async fn delete_note(
    db: &Database,
    space_id: &str,
    remote_id: &str,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    c.delete_space_note(db, space_id, remote_id)
        .await
        .map_err(map_err)?;
    pull_space(db, space_id).await?;
    Ok(())
}
