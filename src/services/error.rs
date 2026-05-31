#[derive(Debug)]
pub enum LlmError {
    NotConfigured(String),
    Embedding(String),
    Completion(String),
    TagParsing(String),
    ReminderParsing(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NotConfigured(msg) => write!(f, "{msg}"),
            LlmError::Embedding(msg) => write!(f, "Embedding error: {msg}"),
            LlmError::Completion(msg) => write!(f, "Completion error: {msg}"),
            LlmError::TagParsing(msg) => write!(f, "Tag parsing error: {msg}"),
            LlmError::ReminderParsing(msg) => {
                write!(f, "Reminder parsing error: {msg}")
            }
        }
    }
}

impl std::error::Error for LlmError {}

impl From<LlmError> for String {
    fn from(err: LlmError) -> Self {
        err.to_string()
    }
}
