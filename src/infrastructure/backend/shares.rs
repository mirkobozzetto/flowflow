// Device plane of shared notes/threads (proposal 0001 T13). Codes always
// travel in JSON bodies (the backend keeps them out of request logs); a dead
// code (unknown, revoked, expired) is one indistinguishable 404.

use super::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(serde::Serialize)]
pub struct PublishNote {
    pub title: Option<String>,
    pub content: String,
}

#[derive(serde::Serialize)]
struct PublishReq<'a> {
    kind: &'a str,
    source_id: &'a str,
    title: Option<&'a str>,
    notes: &'a [PublishNote],
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct PublishShareResp {
    pub code: String,
    pub expires_at: String,
}

#[derive(serde::Serialize)]
struct CodeReq<'a> {
    code: &'a str,
}

#[derive(serde::Serialize)]
struct AppendReq<'a> {
    code: &'a str,
    title: Option<&'a str>,
    content: &'a str,
}

#[derive(serde::Serialize)]
struct NoteEditReq<'a> {
    code: &'a str,
    id: &'a str,
    title: Option<&'a str>,
    content: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RemoteAuthor {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub gone: bool,
    // opaque per-author handle for the local block list (App Store 1.2)
    #[serde(default)]
    pub author_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RemoteSharedNote {
    pub id: String,
    pub author: RemoteAuthor,
    pub own: bool,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RemoteThread {
    pub code: String,
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    pub owner: RemoteAuthor,
    pub own_thread: bool,
    pub created_at: String,
    pub expires_at: String,
    pub notes: Vec<RemoteSharedNote>,
}

#[derive(serde::Deserialize)]
struct NoteIdResp {
    id: String,
}

impl BackendError {
    // The uniform dead-code answer: unknown, revoked and expired all read 404.
    pub fn is_not_found(&self) -> bool {
        matches!(self, BackendError::Status(404, _))
    }
}

impl BackendClient {
    /// Publish a note or thread. Server enforces premium + linked web account
    /// + quota + the 64 KB per-note cap; expiry defaults server-side.
    pub async fn publish_share(
        &self,
        db: &Database,
        kind: &str,
        source_id: &str,
        title: Option<&str>,
        notes: &[PublishNote],
    ) -> Result<PublishShareResp, BackendError> {
        let url = format!("{}/v1/shares", self.base_url);
        let body = PublishReq {
            kind,
            source_id,
            title,
            notes,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Open a code: the thread, its notes, and which of them are mine.
    pub async fn read_share(
        &self,
        db: &Database,
        code: &str,
    ) -> Result<RemoteThread, BackendError> {
        let url = format!("{}/v1/shares/read", self.base_url);
        let body = CodeReq { code };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::read_json(resp).await
    }

    /// Append my own note to a shared thread. Returns the remote note id.
    pub async fn append_share_note(
        &self,
        db: &Database,
        code: &str,
        title: Option<&str>,
        content: &str,
    ) -> Result<String, BackendError> {
        let url = format!("{}/v1/shares/append", self.base_url);
        let body = AppendReq {
            code,
            title,
            content,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        let parsed: NoteIdResp = Self::read_json(resp).await?;
        Ok(parsed.id)
    }

    /// Edit my own note in a shared thread.
    pub async fn update_share_note(
        &self,
        db: &Database,
        code: &str,
        id: &str,
        title: Option<&str>,
        content: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/shares/note/update", self.base_url);
        let body = NoteEditReq {
            code,
            id,
            title,
            content: Some(content),
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Tombstone my own note: the deletion signal every reader phone applies.
    pub async fn delete_share_note(
        &self,
        db: &Database,
        code: &str,
        id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/shares/note/delete", self.base_url);
        let body = NoteEditReq {
            code,
            id,
            title: None,
            content: None,
        };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Kill my code: readers get the uniform 404 from now on.
    pub async fn revoke_share(
        &self,
        db: &Database,
        code: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/shares/revoke", self.base_url);
        let body = CodeReq { code };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Report a share (authenticated, deduplicated server-side, never auto-hides).
    pub async fn report_share(
        &self,
        db: &Database,
        code: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/shares/report", self.base_url);
        let body = CodeReq { code };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Tombstone EVERY shared note I authored, everywhere (lifecycle rule 5:
    /// run before deleting the account).
    pub async fn delete_my_shared_notes(
        &self,
        db: &Database,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/shares/delete-my-notes", self.base_url);
        let resp = self.authed(db, |c, t| c.post(&url).bearer_auth(t)).await?;
        Self::expect_success(resp).await
    }
}
