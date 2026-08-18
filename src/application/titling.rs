use crate::application::constants::{CHEAP_MODEL, TITLE_SYSTEM_PROMPT};
use crate::application::error::LlmError;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;

pub async fn generate_title(
    client: &LlmClient,
    content: &str,
    lang: &str,
) -> Result<String, LlmError> {
    let lang_name = if lang == "fr" { "French" } else { "English" };
    let system =
        format!("{TITLE_SYSTEM_PROMPT}\n\nRespond ONLY in {lang_name}.");
    let preview: String = content.chars().take(1500).collect();
    let response = client
        .chat_with_model(CHEAP_MODEL, &system, &preview)
        .await?;
    let title = response.trim().trim_matches('"').trim().to_string();
    if title.is_empty() {
        return Err(LlmError::Completion("Empty title".into()));
    }
    Ok(title)
}

/// Replace a conversation's placeholder title with a real one once the first
/// answer lands. Returns the new title, or `None` when nothing was written.
///
/// The stored title must still equal `placeholder`: anything else means the
/// user renamed the conversation by hand, and a manual rename is final.
pub async fn retitle_conversation(
    db: &Database,
    conv_id: &str,
    placeholder: &str,
    content: &str,
    lang: &str,
) -> Option<String> {
    let current = db
        .list_conversations()
        .ok()?
        .into_iter()
        .find(|c| c.id == conv_id)?
        .title;
    if current != placeholder {
        return None;
    }
    let client = LlmClient::from_db(db).ok()?;
    let title = generate_title(&client, content, lang).await.ok()?;
    db.update_conversation_title(conv_id, &title).ok()?;
    Some(title)
}
