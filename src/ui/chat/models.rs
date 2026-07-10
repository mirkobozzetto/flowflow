#[derive(Clone, PartialEq)]
pub struct ChatSource {
    pub note_id: String,
    pub title: String,
    pub chunk_text: String,
    pub distance: f32,
    pub created_at: String,
    pub url: Option<String>,
}

use crate::application::approvals::{ProposalView, ReloadStatus};
use crate::application::tools::ProposalStatus as ResolvedStatus;

#[derive(Clone, PartialEq)]
pub enum ChatMsg {
    User(String),
    Bot {
        text: String,
        sources: Vec<ChatSource>,
        /// RAG execution trace, shown only under the Settings debug toggle.
        trace: Option<crate::application::rag::RagTrace>,
    },
    /// A suspended connector write awaiting the user's decision, rendered as the approval card.
    /// `status` drives the card (buttons while `Pending`, a frozen pill after); `edit_error`
    /// carries the gate's reason when a submitted edit fails re-validation.
    Proposal {
        view: ProposalView,
        status: ProposalStatus,
        edit_error: Option<String>,
    },
}

/// The card's lifecycle as the chat renders it. `Pending` is the live state; the rest are
/// frozen outcomes. Mirrors `tools::ProposalStatus`, plus the pre-decision `Pending` that the
/// resolved event never carries.
#[derive(Clone, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Edited,
    Rejected,
    Expired,
}

impl From<ResolvedStatus> for ProposalStatus {
    fn from(s: ResolvedStatus) -> Self {
        match s {
            ResolvedStatus::Approved => Self::Approved,
            ResolvedStatus::Edited => Self::Edited,
            ResolvedStatus::Rejected => Self::Rejected,
            ResolvedStatus::Expired => Self::Expired,
        }
    }
}

impl From<ReloadStatus> for ProposalStatus {
    fn from(s: ReloadStatus) -> Self {
        match s {
            ReloadStatus::Approved => Self::Approved,
            ReloadStatus::Edited => Self::Edited,
            ReloadStatus::Rejected => Self::Rejected,
            ReloadStatus::Expired => Self::Expired,
        }
    }
}

use crate::application::i18n::t;

pub fn tool_label(lang: &str, name: &str) -> String {
    let key = match name {
        "search_notes" => "chat-tool-search",
        "create_note" => "chat-tool-create",
        "summarize_folder" => "chat-tool-summarize",
        "web_search" => "chat-tool-web",
        _ => "chat-tool-working",
    };
    t(lang, key)
}
