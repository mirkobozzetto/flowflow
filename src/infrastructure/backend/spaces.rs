// Device plane of collaborative spaces (proposal 0002 T08). A space is a live
// folder tree pulled by cursor, not a snapshot behind a code.
//
// Two backend disciplines shape this client:
//   - Uniform 404. Unknown space, never joined, removed member and revoked
//     space are one indistinguishable answer, surfaced as `is_not_found()`
//     rather than as a hard failure.
//   - Writes freeze, they never disappear. When the space OWNER stops paying,
//     every write answers 409 `space_read_only` while `pull` keeps serving.
//     `is_read_only()` names that state so the UI can say it.
//
// A space voice note travels as its transcription only: `space_notes` carries
// text, and no audio file crosses the backend (proposal §8 Q4).

use super::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct SpaceResp {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct InviteResp {
    pub code: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RemoteFolder {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub name: String,
    pub mode: String,
    // resolved server-side by walking the ancestor chain; never recomputed here
    pub effective_mode: String,
    #[serde(default)]
    pub author_ref: Option<String>,
    pub seq: i64,
    pub updated_at: String,
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RemoteSpaceNote {
    pub id: String,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub author_ref: Option<String>,
    pub own: bool,
    pub seq: i64,
    pub updated_at: String,
    pub deleted: bool,
    // NULL on a tombstone: the row survives to carry the deletion, its content
    // does not
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RemoteThread {
    pub id: String,
    #[serde(default)]
    pub folder_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub author_ref: Option<String>,
    pub own: bool,
    pub seq: i64,
    pub updated_at: String,
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct PullResp {
    pub folders: Vec<RemoteFolder>,
    pub notes: Vec<RemoteSpaceNote>,
    // absent from a backend that predates threads
    #[serde(default)]
    pub threads: Vec<RemoteThread>,
    pub next_seq: i64,
    // still catching up: pull again at once instead of waiting out the 30 s
    // floor, which only guards steady-state polling
    pub more: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct MemberResp {
    pub web_user_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub author_ref: String,
    pub is_owner: bool,
    #[serde(default)]
    pub is_agent: bool,
    pub me: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct AgentView {
    pub agent_id: String,
    pub name: String,
    pub scope: String,
    pub expires_at: String,
    #[serde(default)]
    pub revoked_at: Option<String>,
    #[serde(default)]
    pub last_used_at: Option<String>,
    pub last_ack_seq: i64,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct AgentCreateResp {
    pub agent_id: String,
    pub token_id: String,
    pub token: String,
    pub scope: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct AgentTokenResp {
    pub token_id: String,
    pub token: String,
    pub scope: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct IdResp {
    pub id: String,
    pub seq: i64,
}

#[derive(serde::Serialize)]
struct CreateReq<'a> {
    name: &'a str,
}

#[derive(serde::Serialize)]
struct SpaceReq<'a> {
    space_id: &'a str,
}

#[derive(serde::Serialize)]
struct RenameReq<'a> {
    space_id: &'a str,
    name: &'a str,
}

#[derive(serde::Serialize)]
struct JoinReq<'a> {
    code: &'a str,
}

#[derive(serde::Serialize)]
struct PullReq<'a> {
    space_id: &'a str,
    since_seq: i64,
}

#[derive(serde::Serialize)]
struct FolderReq<'a> {
    space_id: &'a str,
    id: Option<&'a str>,
    parent_id: Option<&'a str>,
    name: &'a str,
    mode: &'a str,
}

#[derive(serde::Serialize)]
struct FolderMoveReq<'a> {
    space_id: &'a str,
    id: &'a str,
    parent_id: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct IdReq<'a> {
    space_id: &'a str,
    id: &'a str,
}

#[derive(serde::Serialize)]
struct NoteReq<'a> {
    space_id: &'a str,
    id: Option<&'a str>,
    folder_id: Option<&'a str>,
    thread_id: Option<&'a str>,
    title: Option<&'a str>,
    content: &'a str,
}

#[derive(serde::Serialize)]
struct ThreadReq<'a> {
    space_id: &'a str,
    id: Option<&'a str>,
    folder_id: Option<&'a str>,
    title: &'a str,
}

#[derive(serde::Serialize)]
struct MemberRemoveReq<'a> {
    space_id: &'a str,
    web_user_id: &'a str,
}

#[derive(serde::Serialize)]
struct AgentCreateReq<'a> {
    space_id: &'a str,
    name: &'a str,
    scope: &'a str,
    ttl_days: Option<i64>,
}

#[derive(serde::Serialize)]
struct AgentTokenReq<'a> {
    space_id: &'a str,
    agent_id: &'a str,
    scope: &'a str,
    ttl_days: Option<i64>,
}

#[derive(serde::Serialize)]
struct AgentRevokeReq<'a> {
    space_id: &'a str,
    agent_id: &'a str,
}

#[derive(serde::Serialize)]
struct LeaveReq<'a> {
    space_id: &'a str,
    withdraw_notes: bool,
}

impl BackendError {
    /// The owner stopped paying: the space is frozen read-only, not gone.
    pub fn is_read_only(&self) -> bool {
        matches!(self, BackendError::Status(409, b) if b.contains("space_read_only"))
    }

    /// A cap was hit (members, notes). The body names which one.
    pub fn is_limit(&self) -> bool {
        matches!(self, BackendError::Status(409, b)
            if b.contains("space_member_limit") || b.contains("space_note_limit"))
    }
}

impl BackendClient {
    /// Create a space. The caller becomes owner, so premium IS tested here and
    /// never again on anyone they invite.
    pub async fn create_space(
        &self,
        db: &Database,
        name: &str,
    ) -> Result<SpaceResp, BackendError> {
        let url = format!("{}/v1/spaces", self.base_url);
        let body = CreateReq { name };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Mint a single-use invite code (owner only).
    pub async fn invite_to_space(
        &self,
        db: &Database,
        space_id: &str,
    ) -> Result<InviteResp, BackendError> {
        let url = format!("{}/v1/spaces/invite", self.base_url);
        let body = SpaceReq { space_id };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Consume an invite code and become a member. No premium test on the
    /// joiner: that is the whole point of the plane.
    pub async fn join_space(
        &self,
        db: &Database,
        code: &str,
    ) -> Result<SpaceResp, BackendError> {
        let url = format!("{}/v1/spaces/join", self.base_url);
        let body = JoinReq { code };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// The only read route, and the only one that costs. Everything changed
    /// after `since_seq`, tombstones included, in cursor order.
    pub async fn pull_space(
        &self,
        db: &Database,
        space_id: &str,
        since_seq: i64,
    ) -> Result<PullResp, BackendError> {
        let url = format!("{}/v1/spaces/pull", self.base_url);
        let body = PullReq {
            space_id,
            since_seq,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Create (`id` = None) or rename/remode (`id` = Some) a folder.
    pub async fn put_space_folder(
        &self,
        db: &Database,
        space_id: &str,
        id: Option<&str>,
        parent_id: Option<&str>,
        name: &str,
        mode: &str,
    ) -> Result<IdResp, BackendError> {
        let url = format!("{}/v1/spaces/folder", self.base_url);
        let body = FolderReq {
            space_id,
            id,
            parent_id,
            name,
            mode,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Reparent a folder. `parent_id` None moves it to the space root; a move
    /// under one's own descendant answers 409 `folder_cycle`.
    pub async fn move_space_folder(
        &self,
        db: &Database,
        space_id: &str,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/folder/move", self.base_url);
        let body = FolderMoveReq {
            space_id,
            id,
            parent_id,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Tombstone a folder AND its whole subtree, notes included.
    pub async fn delete_space_folder(
        &self,
        db: &Database,
        space_id: &str,
        id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/folder/delete", self.base_url);
        let body = IdReq { space_id, id };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Create (`id` = None) or update one's OWN note (`id` = Some). Text only.
    /// `thread_id` must name a live thread of the same space, or be None.
    #[allow(clippy::too_many_arguments)]
    pub async fn put_space_note(
        &self,
        db: &Database,
        space_id: &str,
        id: Option<&str>,
        folder_id: Option<&str>,
        thread_id: Option<&str>,
        title: Option<&str>,
        content: &str,
    ) -> Result<IdResp, BackendError> {
        let url = format!("{}/v1/spaces/note", self.base_url);
        let body = NoteReq {
            space_id,
            id,
            folder_id,
            thread_id,
            title,
            content,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Create (`id` = None) or update one's OWN thread (`id` = Some).
    pub async fn put_space_thread(
        &self,
        db: &Database,
        space_id: &str,
        id: Option<&str>,
        folder_id: Option<&str>,
        title: &str,
    ) -> Result<IdResp, BackendError> {
        let url = format!("{}/v1/spaces/thread", self.base_url);
        let body = ThreadReq {
            space_id,
            id,
            folder_id,
            title,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Tombstone one's OWN thread. Member notes survive, detached.
    pub async fn delete_space_thread(
        &self,
        db: &Database,
        space_id: &str,
        id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/thread/delete", self.base_url);
        let body = IdReq { space_id, id };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Tombstone one's OWN note: the deletion signal every other device applies
    /// (drop the local note, then purge its vectors).
    pub async fn delete_space_note(
        &self,
        db: &Database,
        space_id: &str,
        id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/note/delete", self.base_url);
        let body = IdReq { space_id, id };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Who is in the space, as any member may ask.
    pub async fn list_space_members(
        &self,
        db: &Database,
        space_id: &str,
    ) -> Result<Vec<MemberResp>, BackendError> {
        let url = format!("{}/v1/spaces/members", self.base_url);
        let body = SpaceReq { space_id };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Owner renames the space.
    pub async fn rename_space(
        &self,
        db: &Database,
        space_id: &str,
        name: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/rename", self.base_url);
        let body = RenameReq { space_id, name };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Owner stops sharing: the space answers 404 to everyone from here on.
    pub async fn delete_space(
        &self,
        db: &Database,
        space_id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/delete", self.base_url);
        let body = SpaceReq { space_id };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Owner revokes a member. Their notes stay: forcing their removal is an
    /// open legal question, only the departing author may withdraw them.
    pub async fn remove_space_member(
        &self,
        db: &Database,
        space_id: &str,
        web_user_id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/member/remove", self.base_url);
        let body = MemberRemoveReq {
            space_id,
            web_user_id,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Leave a space, optionally tombstoning everything one wrote in it.
    pub async fn leave_space(
        &self,
        db: &Database,
        space_id: &str,
        withdraw_notes: bool,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/leave", self.base_url);
        let body = LeaveReq {
            space_id,
            withdraw_notes,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    pub async fn create_space_agent(
        &self,
        db: &Database,
        space_id: &str,
        name: &str,
        scope: &str,
        ttl_days: Option<i64>,
    ) -> Result<AgentCreateResp, BackendError> {
        let url = format!("{}/v1/spaces/agent", self.base_url);
        let body = AgentCreateReq {
            space_id,
            name,
            scope,
            ttl_days,
        };
        let response = self
            .authed(db, |client, token| {
                client.post(&url).bearer_auth(token).json(&body)
            })
            .await?;
        Self::read_json(response).await
    }

    pub async fn list_space_agents(
        &self,
        db: &Database,
        space_id: &str,
    ) -> Result<Vec<AgentView>, BackendError> {
        let url = format!("{}/v1/spaces/agents", self.base_url);
        let body = SpaceReq { space_id };
        let response = self
            .authed(db, |client, token| {
                client.post(&url).bearer_auth(token).json(&body)
            })
            .await?;
        Self::read_json(response).await
    }

    pub async fn rotate_space_agent_token(
        &self,
        db: &Database,
        space_id: &str,
        agent_id: &str,
        scope: &str,
        ttl_days: Option<i64>,
    ) -> Result<AgentTokenResp, BackendError> {
        let url = format!("{}/v1/spaces/agent/token", self.base_url);
        let body = AgentTokenReq {
            space_id,
            agent_id,
            scope,
            ttl_days,
        };
        let response = self
            .authed(db, |client, token| {
                client.post(&url).bearer_auth(token).json(&body)
            })
            .await?;
        Self::read_json(response).await
    }

    pub async fn revoke_space_agent(
        &self,
        db: &Database,
        space_id: &str,
        agent_id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/spaces/agent/revoke", self.base_url);
        let body = AgentRevokeReq { space_id, agent_id };
        let response = self
            .authed(db, |client, token| {
                client.post(&url).bearer_auth(token).json(&body)
            })
            .await?;
        Self::expect_success(response).await
    }
}
