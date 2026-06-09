pub mod conflict;
pub mod engine;
pub mod peers;
pub mod protocol;
pub mod reconcile;
pub mod transport;
pub mod vv;

#[derive(Debug)]
pub enum SyncError {
    Handshake(String),
    Pairing(String),
    Transport(String),
    Protocol(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Handshake(m) => write!(f, "Handshake error: {m}"),
            SyncError::Pairing(m) => write!(f, "Pairing error: {m}"),
            SyncError::Transport(m) => write!(f, "Transport error: {m}"),
            SyncError::Protocol(m) => write!(f, "Protocol error: {m}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<SyncError> for String {
    fn from(e: SyncError) -> Self {
        e.to_string()
    }
}
