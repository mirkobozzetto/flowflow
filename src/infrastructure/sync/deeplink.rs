use std::sync::Mutex;
use std::sync::OnceLock;

static PENDING: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn cell() -> &'static Mutex<Option<String>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

pub fn push(uri: String) {
    if let Ok(mut guard) = cell().lock() {
        *guard = Some(uri);
    }
}

pub fn peek() -> Option<String> {
    cell().lock().ok().and_then(|guard| guard.clone())
}

pub fn take() -> Option<String> {
    cell().lock().ok().and_then(|mut guard| guard.take())
}

/// Prefix-scoped mailbox reads: several consumers poll this single slot
/// (pairing view, record watcher), each must only see its own URIs or one
/// would swallow the other's deep link.
pub fn peek_matching(prefix: &str) -> Option<String> {
    peek().filter(|uri| uri.starts_with(prefix))
}

pub fn take_matching(prefix: &str) -> Option<String> {
    let mut guard = cell().lock().ok()?;
    if guard
        .as_ref()
        .map(|uri| uri.starts_with(prefix))
        .unwrap_or(false)
    {
        guard.take()
    } else {
        None
    }
}
