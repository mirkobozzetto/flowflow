// Shared notes/threads (proposal 0001): the device-side records. A LocalShare
// is one of MY live publications, keyed by the local source (note or thread).
// A Provenance is the frozen origin of a note kept from someone else's share.

#[derive(Debug, Clone, PartialEq)]
pub enum ShareKind {
    Note,
    Thread,
}

impl ShareKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShareKind::Note => "note",
            ShareKind::Thread => "thread",
        }
    }

    pub fn parse(s: &str) -> Option<ShareKind> {
        match s {
            "note" => Some(ShareKind::Note),
            "thread" => Some(ShareKind::Thread),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalShare {
    pub source_id: String,
    pub kind: ShareKind,
    pub code: String,
    pub expires_at: String,
    pub created_at: String,
}

pub const PROVENANCE_LIVE: &str = "live";
pub const PROVENANCE_GONE: &str = "gone";

#[derive(Debug, Clone)]
pub struct Provenance {
    pub note_id: String,
    pub share_code: String,
    pub remote_note_id: String,
    pub author_name: Option<String>,
    pub captured_at: String,
    // "live" while the source still answers; "gone" greys the author out.
    pub state: String,
}

// The deep-link scheme a share travels in: flowflow://share/{code}.
pub const SHARE_LINK_PREFIX: &str = "flowflow://share/";

pub fn share_link(code: &str) -> String {
    format!("{SHARE_LINK_PREFIX}{code}")
}

pub fn parse_share_link(link: &str) -> Option<&str> {
    let code = link.trim().strip_prefix(SHARE_LINK_PREFIX)?;
    (!code.is_empty() && code.len() <= 64 && !code.contains('/'))
        .then_some(code)
}
