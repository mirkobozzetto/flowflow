use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub folder_id: Option<String>,
    pub created_at: String,
    pub modified_at: String,
}

pub struct NewThread {
    pub title: String,
    pub folder_id: Option<String>,
}

pub struct UpdateThread {
    pub title: Option<String>,
    pub folder_id: Option<Option<String>>,
}
