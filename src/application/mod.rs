pub mod ai;
pub mod backup;
pub mod constants;
pub mod embed;
pub mod error;
pub mod i18n;
pub mod intent;
pub mod rag;
pub mod reminders;
pub mod tools;
pub mod web_search;

pub use embed::{
    delete_attachment_embeddings, delete_note_embeddings, embed_attachment,
    embed_note,
};
pub use error::LlmError;
pub use rag::{RagResponse, RagSource};
