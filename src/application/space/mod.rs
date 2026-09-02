// Collaborative spaces, app side (proposal 0002 T09/T10/T14). A space is a
// live folder tree the server owns; this device holds a mirror it refreshes by
// cursor.
//
// Three rules run through the module:
//   - The server is the authority. A pulled row overwrites the local copy, it
//     is never merged with it.
//   - A write while offline is REFUSED, never queued. A queue would reopen the
//     question of who decides, which is exactly what server authority closes.
//     One bounded exception: a note the editor already saved into a space
//     folder is retried until the server has it (`space_publish_pending`),
//     because the alternative is a note that silently never leaves the device.
//   - A space note is an ORDINARY local note. It goes through the same repos
//     and the same embed pipeline, so it is searchable and usable in chat with
//     no code of its own.

mod adopt;
mod pull;
mod write;

pub use adopt::{resume_adoptions, share_existing_folder, ShareTarget};

pub use pull::{
    apply_delta, my_author_ref, pull_all_due, pull_if_due, pull_space,
    republish_pending, PageEffects, PullOutcome,
};
pub use write::{
    can_write, create_folder, create_note, delete_folder, delete_note,
    folder_right, move_folder, publish_local_note, update_folder, update_note,
    FolderRight,
};

use crate::infrastructure::backend::spaces::{
    AgentCreateResp, AgentTokenResp, AgentView, MemberResp,
};
use crate::infrastructure::backend::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(Debug, Clone, PartialEq)]
pub enum SpaceError {
    // no backend configured: the feature is dark
    NoBackend,
    // no network. The write did NOT happen and is not queued.
    Offline,
    // 403: premium or a linked web account is missing (the backend does not
    // say which, on purpose)
    Refused,
    // the uniform 404: unknown space, never joined, removed member, revoked
    Gone,
    // the OWNER stopped paying: the space is frozen read-only, not lost
    ReadOnly,
    // a cap was hit (members, notes, note size)
    Limit(String),
    Other(String),
}

impl std::fmt::Display for SpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpaceError::NoBackend => write!(f, "no backend configured"),
            SpaceError::Offline => write!(f, "offline"),
            SpaceError::Refused => {
                write!(f, "refused (premium + linked account required)")
            }
            SpaceError::Gone => write!(f, "space not found"),
            SpaceError::ReadOnly => {
                write!(f, "space is read-only (owner subscription lapsed)")
            }
            SpaceError::Limit(w) => write!(f, "limit reached: {w}"),
            SpaceError::Other(e) => write!(f, "{e}"),
        }
    }
}

/// The i18n key a screen shows for an error. One key per variant and no
/// wildcard arm, so a new variant cannot silently fall back to "-other".
pub fn error_key(e: &SpaceError) -> &'static str {
    match e {
        SpaceError::NoBackend => "space-error-no-backend",
        SpaceError::Offline => "space-error-offline",
        SpaceError::Refused => "space-error-refused",
        SpaceError::Gone => "space-error-gone",
        SpaceError::ReadOnly => "space-error-read-only",
        SpaceError::Limit(_) => "space-error-limit",
        SpaceError::Other(_) => "space-error-other",
    }
}

pub(super) fn map_err(e: BackendError) -> SpaceError {
    match e {
        BackendError::Network(_) => SpaceError::Offline,
        BackendError::Status(404, _) => SpaceError::Gone,
        BackendError::Status(403, _) | BackendError::Unauthorized => {
            SpaceError::Refused
        }
        ref s if s.is_read_only() => SpaceError::ReadOnly,
        ref s if s.is_limit() => SpaceError::Limit(s.to_string()),
        other => SpaceError::Other(other.to_string()),
    }
}

pub(super) fn client(db: &Database) -> Result<BackendClient, SpaceError> {
    BackendClient::from_db(db).ok_or(SpaceError::NoBackend)
}

/// Create a space. Premium is tested on the caller HERE and never again on
/// anyone they invite: creating costs, joining does not.
pub async fn create_space(
    db: &Database,
    name: &str,
) -> Result<String, SpaceError> {
    let c = client(db)?;
    let resp = c.create_space(db, name).await.map_err(map_err)?;
    db.upsert_space(&resp.id, &resp.name, true)
        .map_err(SpaceError::Other)?;
    Ok(resp.id)
}

/// Mint a single-use invite code (owner only).
pub async fn invite(
    db: &Database,
    space_id: &str,
) -> Result<String, SpaceError> {
    let c = client(db)?;
    c.invite_to_space(db, space_id)
        .await
        .map(|r| r.code)
        .map_err(map_err)
}

/// The invite as the LINK to hand over: what gets pasted into a message has
/// to be tappable.
pub async fn invite_link(
    db: &Database,
    space_id: &str,
) -> Result<String, SpaceError> {
    invite(db, space_id)
        .await
        .map(|code| crate::domain::space::space_link(&code))
}

/// Who is in the space.
pub async fn members(
    db: &Database,
    space_id: &str,
) -> Result<Vec<MemberResp>, SpaceError> {
    let c = client(db)?;
    c.list_space_members(db, space_id).await.map_err(map_err)
}

pub async fn create_agent(
    db: &Database,
    space_id: &str,
    name: &str,
    scope: &str,
) -> Result<AgentCreateResp, SpaceError> {
    let client = client(db)?;
    client
        .create_space_agent(db, space_id, name, scope, None)
        .await
        .map_err(map_err)
}

pub async fn agents(
    db: &Database,
    space_id: &str,
) -> Result<Vec<AgentView>, SpaceError> {
    let client = client(db)?;
    client
        .list_space_agents(db, space_id)
        .await
        .map_err(map_err)
}

pub async fn rotate_agent_token(
    db: &Database,
    space_id: &str,
    agent_id: &str,
    scope: &str,
) -> Result<AgentTokenResp, SpaceError> {
    let client = client(db)?;
    client
        .rotate_space_agent_token(db, space_id, agent_id, scope, None)
        .await
        .map_err(map_err)
}

pub async fn revoke_agent(
    db: &Database,
    space_id: &str,
    agent_id: &str,
) -> Result<(), SpaceError> {
    let client = client(db)?;
    client
        .revoke_space_agent(db, space_id, agent_id)
        .await
        .map_err(map_err)
}

/// Owner renames the space, server first, then the local row.
pub async fn rename(
    db: &Database,
    space_id: &str,
    name: &str,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    c.rename_space(db, space_id, name).await.map_err(map_err)?;
    db.upsert_space(space_id, name, true)
        .map_err(SpaceError::Other)
}

/// Owner stops sharing. Server first; then, like a member who finds the
/// space gone, this device keeps everything as ordinary local notes and
/// themes: nothing the owner wrote or received is destroyed.
pub async fn stop_sharing(
    db: &Database,
    space_id: &str,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    match c.delete_space(db, space_id).await {
        Ok(()) => {}
        // already gone server-side: the local outcome is the same
        Err(e) if e.is_not_found() => {}
        Err(e) => return Err(map_err(e)),
    }
    detach_locally(db, space_id, Departure::Revoked);
    Ok(())
}

/// Consume an invite code, then pull the whole space in: joining with an empty
/// tree would show a space that looks broken.
pub async fn join(db: &Database, code: &str) -> Result<String, SpaceError> {
    let c = client(db)?;
    let resp = c.join_space(db, code).await.map_err(map_err)?;
    db.upsert_space(&resp.id, &resp.name, false)
        .map_err(SpaceError::Other)?;
    pull_space(db, &resp.id).await?;
    Ok(resp.id)
}

/// What happens to the notes of someone walking out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Departure {
    /// Revoked, or the space was deleted. Nothing is destroyed: everything
    /// already on this device becomes an ordinary local note, authors included
    /// (proposal §6.6 - only the departing author may ever withdraw content,
    /// and this device did not choose to leave).
    Revoked,
    /// Copy my notes into ordinary local ones and drop the space mirror. The
    /// copies wait for no pull; their local ids do not change, so their
    /// embeddings stay valid and nothing needs re-purging.
    KeepMine,
    /// Withdraw everything I wrote: it is tombstoned server-side and vanishes
    /// for every other member too.
    WithdrawMine,
}

/// Leave a space and clean up locally.
///
/// A note written by someone ELSE is dropped either way: it belongs to a space
/// this device no longer reads, and keeping a stale copy indexed is the ghost
/// note the whole design exists to prevent.
pub async fn leave(
    db: &Database,
    space_id: &str,
    departure: Departure,
) -> Result<(), SpaceError> {
    let c = client(db)?;
    match c
        .leave_space(db, space_id, departure == Departure::WithdrawMine)
        .await
    {
        Ok(()) => {}
        // already removed server-side (revoked): the local cleanup still runs,
        // that is the outcome the user asked for
        Err(e) if e.is_not_found() => {}
        Err(e) => return Err(map_err(e)),
    }
    detach_locally(db, space_id, departure);
    Ok(())
}

/// Local half of leaving, also used when a pull discovers the membership is
/// gone (revoked owner-side, no leave call of ours).
pub fn detach_locally(db: &Database, space_id: &str, departure: Departure) {
    let keep_all = departure == Departure::Revoked;
    let mine_stay = keep_all || departure == Departure::KeepMine;
    let me = my_author_ref(db);
    for note_id in db.space_note_ids(space_id).unwrap_or_default() {
        let own = db
            .get_note(&note_id)
            .ok()
            .flatten()
            .map(|n| n.author_ref.is_some() && n.author_ref == me)
            .unwrap_or(false);
        if keep_all || (own && mine_stay) {
            let _ = db.detach_note_from_space(&note_id);
        } else {
            crate::application::note_persistence::delete_note(db, &note_id);
        }
    }
    for folder_id in db.space_folder_ids(space_id).unwrap_or_default() {
        if mine_stay {
            let _ = db.detach_folder_from_space(&folder_id);
        } else {
            let _ = db.delete_folder(&folder_id);
        }
    }
    let _ = db.delete_space(space_id);
}

/// Owner revokes a member.
pub async fn remove_member(
    db: &Database,
    space_id: &str,
    web_user_id: &str,
) -> Result<(), SpaceError> {
    if let Some(agent_id) = web_user_id.strip_prefix("agent:") {
        return revoke_agent(db, space_id, agent_id).await;
    }
    let client = client(db)?;
    client
        .remove_space_member(db, space_id, web_user_id)
        .await
        .map_err(map_err)
}
