//! Hermes' task results delivered to FlowFlow. Hermes' bundled ntfy module
//! publishes each result to the FlowFlow server, which alerts the iPhone:
//! nothing to install on Hermes, nothing for the person to type.

use crate::application::hermes_jobs::{self, Frequency};
use crate::application::push::{self, PushError};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::hermes::{HermesClient, HermesError, HermesJob};
use crate::infrastructure::persistence::Database;
use chrono::TimeZone;
use sha2::{Digest, Sha256};

/// Hermes' name for its ntfy module, as a destination in `deliver`.
pub const FLOWFLOW: &str = "ntfy";
/// Hermes keeps the answer and sends it nowhere.
const KEPT: &str = "local";
const TOPIC_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryError {
    Push(PushError),
    Hermes(HermesError),
}

impl From<PushError> for DeliveryError {
    fn from(e: PushError) -> Self {
        DeliveryError::Push(e)
    }
}

impl From<HermesError> for DeliveryError {
    fn from(e: HermesError) -> Self {
        DeliveryError::Hermes(e)
    }
}

/// `deliver` with FlowFlow added or taken out, the other destinations kept.
pub fn with_flowflow(deliver: &str, on: bool) -> String {
    let mut parts: Vec<&str> = deliver
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != KEPT && *p != FLOWFLOW)
        .collect();
    if on {
        parts.push(FLOWFLOW);
    }
    if parts.is_empty() {
        KEPT.to_string()
    } else {
        parts.join(",")
    }
}

pub fn delivers_to_flowflow(job: &HermesJob) -> bool {
    job.deliver.split(',').any(|p| p.trim() == FLOWFLOW)
}

/// Whether Telegram gets the answer too: named, or the chat the job was
/// made in.
pub fn telegram_too(job: &HermesJob) -> bool {
    let from_telegram = job
        .origin
        .as_ref()
        .is_some_and(|o| o.platform == "telegram");
    job.deliver.split(',').map(str::trim).any(|p| {
        p == "telegram"
            || p.starts_with("telegram:")
            || (p == "origin" && from_telegram)
    })
}

/// Where Hermes publishes and the token it shows. Every device derives the
/// same pair from the key it shares with Hermes: nothing new travels,
/// nothing is stored, setting it twice changes nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct NtfyLink {
    pub topic: String,
    pub token: String,
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

impl NtfyLink {
    pub fn from_key(key: &str) -> Self {
        let mut topic = sha256_hex(&format!("flowflow-ntfy-topic:{key}"));
        topic.truncate(TOPIC_LEN);
        Self {
            topic,
            token: sha256_hex(&format!("flowflow-ntfy-token:{key}")),
        }
    }

    pub fn token_hash(&self) -> String {
        sha256_hex(&self.token)
    }
}

/// The settings of Hermes' ntfy module, pointed at the FlowFlow server.
pub fn hermes_env(
    link: &NtfyLink,
    backend_base: &str,
) -> Vec<(&'static str, String)> {
    vec![
        (
            "NTFY_SERVER_URL",
            format!("{}/ntfy", backend_base.trim_end_matches('/')),
        ),
        ("NTFY_TOPIC", link.topic.clone()),
        ("NTFY_TOKEN", link.token.clone()),
        ("NTFY_MARKDOWN", "true".to_string()),
        ("NTFY_HOME_CHANNEL", link.topic.clone()),
    ]
}

/// Notifications on, the link known to the server. True when Hermes
/// already listens, so its settings need no write.
async fn ready(
    db: &Database,
    hermes: &HermesClient,
) -> Result<(BackendClient, NtfyLink, bool), DeliveryError> {
    if !push::is_enabled(db) {
        push::enable(db).await?;
    }
    let backend = BackendClient::from_db(db).ok_or(PushError::NoBackend)?;
    let link = NtfyLink::from_key(hermes.key());
    let listening = backend
        .link_hermes(db, &link.topic, &link.token_hash())
        .await
        .map_err(PushError::from)?;
    Ok((backend, link, listening))
}

// Last: Hermes' API restarts with its gateway.
// ponytail: a Hermes reconnecting at that very moment gets the same
// settings again and one more restart; read /api/env first if that bites.
async fn settle(
    hermes: &HermesClient,
    backend: &BackendClient,
    link: &NtfyLink,
    listening: bool,
) -> Result<(), DeliveryError> {
    if !listening {
        hermes
            .configure(&hermes_env(link, backend.base_url()))
            .await?;
    }
    Ok(())
}

/// Turns FlowFlow on or off as a destination of `job`.
pub async fn set(
    db: &Database,
    job: &HermesJob,
    on: bool,
) -> Result<HermesJob, DeliveryError> {
    let hermes = HermesClient::from_db(db)?;
    let deliver = with_flowflow(&job.deliver, on);
    if !on {
        return Ok(hermes.update_deliver(&job.id, &deliver).await?);
    }
    let (backend, link, listening) = ready(db, &hermes).await?;
    let updated = hermes.update_deliver(&job.id, &deliver).await?;
    settle(&hermes, &backend, &link, listening).await?;
    Ok(updated)
}

/// Creates a job, delivering to FlowFlow from the start when asked.
pub async fn create<Tz: TimeZone>(
    db: &Database,
    name: &str,
    prompt: &str,
    freq: &Frequency,
    known: &[HermesJob],
    tz: &Tz,
    to_flowflow: bool,
) -> Result<HermesJob, DeliveryError> {
    if !to_flowflow {
        return Ok(hermes_jobs::create(
            db, name, prompt, freq, known, tz, None,
        )
        .await?);
    }
    let hermes = HermesClient::from_db(db)?;
    let (backend, link, listening) = ready(db, &hermes).await?;
    let job =
        hermes_jobs::create(db, name, prompt, freq, known, tz, Some(FLOWFLOW))
            .await?;
    settle(&hermes, &backend, &link, listening).await?;
    Ok(job)
}
