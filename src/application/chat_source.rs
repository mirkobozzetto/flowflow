//! What answers the chat: one source at a time. Picking one never signs the
//! others out.
use crate::infrastructure::persistence::Database;
use crate::infrastructure::{chatgpt_auth, Provider};
use std::str::FromStr;

const PROVIDER_KEY: &str = "llm_provider";

/// Every source, in menu order.
pub fn all() -> Vec<Provider> {
    let mut sources = Vec::new();
    #[cfg(debug_assertions)]
    sources.push(Provider::ClaudeSubscription);
    sources.extend([Provider::Anthropic, Provider::ChatGpt, Provider::OpenAi]);
    sources
}

pub fn active(db: &Database) -> Provider {
    db.get_setting(PROVIDER_KEY)
        .and_then(|value| Provider::from_str(&value).ok())
        .unwrap_or_default()
}

pub fn choose(db: &Database, source: Provider) -> Result<(), String> {
    db.set_setting(PROVIDER_KEY, source.as_str())
}

/// Signed in, or its key saved: it can answer right away.
pub fn is_ready(db: &Database, source: Provider) -> bool {
    match source {
        Provider::OpenAi => has_key(db, "openai_api_key"),
        Provider::Anthropic => has_key(db, "anthropic_api_key"),
        Provider::ChatGpt => chatgpt_auth::is_connected(),
        #[cfg(debug_assertions)]
        Provider::ClaudeSubscription => {
            crate::infrastructure::claude_auth::is_connected()
        }
    }
}

/// After a sign-out: the chat moves to another ready source, the OpenAI key
/// when nothing else is.
pub fn fall_back_from(db: &Database, gone: Provider) -> Result<(), String> {
    if active(db) != gone {
        return Ok(());
    }
    let next = all()
        .into_iter()
        .find(|&source| source != gone && is_ready(db, source))
        .unwrap_or(Provider::OpenAi);
    choose(db, next)
}

fn has_key(db: &Database, key: &str) -> bool {
    db.get_setting(key)
        .is_some_and(|value| !value.trim().is_empty())
}
