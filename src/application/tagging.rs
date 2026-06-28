use crate::application::constants::{CHEAP_MODEL, TAGS_SYSTEM_PROMPT};
use crate::application::error::LlmError;
use crate::infrastructure::llm::LlmClient;

pub async fn generate_tags(
    client: &LlmClient,
    content: &str,
) -> Result<Vec<String>, LlmError> {
    let response = client
        .chat_with_model(CHEAP_MODEL, TAGS_SYSTEM_PROMPT, content)
        .await?;
    parse_tags(&response)
}

pub fn parse_tags(response: &str) -> Result<Vec<String>, LlmError> {
    let trimmed = response.trim();
    serde_json::from_str::<Vec<String>>(trimmed).or_else(|_| {
        if let Some(start) = trimmed.find('[') {
            if let Some(end) = trimmed.rfind(']') {
                return serde_json::from_str(&trimmed[start..=end]).map_err(
                    |e| LlmError::TagParsing(format!("Parse tags: {e}")),
                );
            }
        }
        Err(LlmError::TagParsing(format!(
            "Invalid tags JSON: {trimmed}"
        )))
    })
}
