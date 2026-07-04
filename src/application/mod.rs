pub mod agent_builder;
pub mod agent_directory;
pub mod ai;
pub mod backup;
pub mod chain;
pub mod connector_module;
pub mod constants;
pub mod embed;
pub mod error;
pub mod i18n;
pub mod intent;
pub mod mention;
pub mod note_persistence;
pub mod rag;
pub mod related;
pub mod reminders;
pub mod reminders_extract;
pub mod share_inbox;
pub mod tagging;
pub mod titling;
pub mod tools;
pub mod transcription_manager;
pub mod web_search;

pub use embed::{
    delete_attachment_embeddings, delete_note_embeddings, embed_attachment,
    embed_note,
};
pub use error::LlmError;
pub use rag::{RagResponse, RagSource};
