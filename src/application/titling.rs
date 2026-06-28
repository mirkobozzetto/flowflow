use crate::application::constants::{CHEAP_MODEL, TITLE_SYSTEM_PROMPT};
use crate::application::error::LlmError;
use crate::infrastructure::llm::LlmClient;

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
