pub mod peers;
pub mod reconcile;
pub mod transport;

#[derive(Debug)]
pub enum SyncError {
    Handshake(String),
    Pairing(String),
    Transport(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Handshake(m) => write!(f, "Handshake error: {m}"),
            SyncError::Pairing(m) => write!(f, "Pairing error: {m}"),
            SyncError::Transport(m) => write!(f, "Transport error: {m}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<SyncError> for String {
    fn from(e: SyncError) -> Self {
        e.to_string()
    }
}
