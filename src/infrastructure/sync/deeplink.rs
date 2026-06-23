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
