use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub modified_at: String,
}

pub struct NewFolder {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
}

pub struct UpdateFolder {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Option<String>>,
}
