// Collaborative spaces (proposal 0002). Pure rules, no IO.

use serde::{Deserialize, Serialize};

pub const MODE_READ: &str = "read";
pub const MODE_COLLAB: &str = "collab";

/// A space this device joined, with its pull cursor. Device-local: a cursor is
/// meaningless on another device, so this never travels in sync.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    // True only when THIS device created the space. The pull payload never
    // names the owner, so ownership is recorded at creation rather than
    // inferred from a field the server does not send.
    pub is_owner: bool,
    pub joined_at: String,
    pub cursor: i64,
    pub last_pull_at: Option<String>,
}

/// Shortest gap between two pulls of the same space. Freshness is paid in
/// deltas, not in polling: a pull with nothing to report still costs a round
/// trip, and the server refuses a faster caller anyway.
pub const PULL_FLOOR_SECS: i64 = 30;

/// Has the floor elapsed since the last successful pull? A space never pulled
/// is always due.
pub fn due_for_pull(
    last_pull_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let Some(stamp) = last_pull_at else {
        return true;
    };
    match chrono::DateTime::parse_from_rfc3339(stamp) {
        Ok(t) => {
            (now - t.with_timezone(&chrono::Utc)).num_seconds()
                >= PULL_FLOOR_SECS
        }
        // an unparseable stamp is not a reason to stop refreshing
        Err(_) => true,
    }
}

/// A folder as it exists inside a space: the DECLARED mode plus the parent
/// link. The effective mode is never stored, it is resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceFolder {
    pub id: String,
    pub parent_id: Option<String>,
    pub mode: String,
}

/// Effective mode = `collab` only if this folder AND every ancestor declares
/// `collab`. Making a parent read-only therefore restricts its whole subtree at
/// once, with no child rewritten, and a subtree moved under a read-only parent
/// is restricted the same way.
///
/// The server resolves this on every write; the app resolves it too, so the UI
/// can say "you cannot write here" BEFORE the write fails. A broken chain (an
/// unknown parent, or a cycle beyond `MAX_DEPTH`) resolves to `read`: the safe
/// side of a question we cannot answer.
pub fn effective_mode(folders: &[SpaceFolder], id: &str) -> &'static str {
    const MAX_DEPTH: usize = 8;
    let mut current = Some(id);
    for _ in 0..MAX_DEPTH {
        let Some(cur) = current else {
            // walked off the root with every step collab: the chain is clean
            return MODE_COLLAB;
        };
        let Some(f) = folders.iter().find(|f| f.id == cur) else {
            return MODE_READ;
        };
        if f.mode != MODE_COLLAB {
            return MODE_READ;
        }
        current = f.parent_id.as_deref();
    }
    MODE_READ
}

/// May this actor put content in that folder? `None` is the space root, which
/// only the owner writes into: a member drops notes in a collab folder, never
/// loose at the top of someone else's space.
pub fn can_write_in(
    folders: &[SpaceFolder],
    is_owner: bool,
    folder_id: Option<&str>,
) -> bool {
    if is_owner {
        return true;
    }
    match folder_id {
        None => false,
        Some(id) => effective_mode(folders, id) == MODE_COLLAB,
    }
}
