pub mod account_heal;
pub mod agent_activation;
pub mod agent_builder;
pub mod agent_directory;
pub mod ai;
pub mod approvals;
pub mod authorship;
pub mod backup;
pub mod chain;
pub mod chat_surface;
pub mod connector_module;
pub mod connector_pins;
pub mod constants;
pub mod device_naming;
pub mod embed;
pub mod error;
pub mod i18n;
pub mod intent;
pub mod mention;
pub mod note_persistence;
pub mod profile;
pub mod rag;
pub mod related;
pub mod reminders;
pub mod reminders_extract;
pub mod share_inbox;
pub mod sharing;
pub mod space;
pub mod tagging;
pub mod titling;
pub mod tools;
pub mod transcribe_audio;
pub mod transcription_dictionary;
pub mod transcription_manager;
pub mod web_search;

pub use embed::{
    delete_attachment_embeddings, delete_note_embeddings, embed_attachment,
    embed_note,
};
pub use error::LlmError;
pub use rag::{RagResponse, RagSource};
