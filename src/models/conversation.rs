#[derive(Clone, Debug, PartialEq)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub sources_json: Option<String>,
    pub created_at: String,
}
