use super::proto_err;
use crate::services::sync::transport::SecureChannel;
use crate::services::sync::SyncError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};

// Plaintext routing frame sent BEFORE the Noise handshake: the responder
// needs the right per-peer PSK to build its handshake state. The claim is
// unauthenticated; a lie just selects credentials the liar cannot complete
// the handshake with.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SyncHint {
    pub kind: String,
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum Msg {
    Hello {
        // Missing on builds that predate the field -> 0 -> rejected with a
        // clear "upgrade" error instead of silently dropping unknown kinds.
        #[serde(default)]
        protocol_version: i64,
        device_id: String,
        last_acked_seq: i64,
        gc_horizon: i64,
        // The sender's own seq counter. Lets the receiver detect a RESTORED
        // peer (we acked more of its space than its counter has issued) and
        // force the full-state session both sides will agree on (T19).
        #[serde(default)]
        next_seq: i64,
        #[serde(default)]
        restored: bool,
    },
    RestoredFloor {
        floor: i64,
    },
    Push {
        rows: Vec<SyncRow>,
        done: bool,
    },
    Ack {
        upto_seq: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SyncRow {
    pub entity_kind: String,
    pub entity_id: String,
    pub version_vector: String,
    pub origin_device: String,
    pub origin_seq: i64,
    pub deleted: i64,
    pub updated_hlc: Option<String>,
    pub payload: Option<Value>,
    #[serde(default)]
    pub chunks: Vec<ChunkPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ChunkPayload {
    pub id: String,
    pub chunk_index: i64,
    pub vector_b64: String,
    pub content_hash: Option<String>,
    pub chunk_text: Option<String>,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncStats {
    pub pushed: usize,
    pub applied: usize,
    pub skipped: usize,
    pub conflicts: usize,
}

pub(super) fn send_msg<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    msg: &Msg,
) -> Result<(), SyncError> {
    let raw = serde_json::to_vec(msg)
        .map_err(|e| proto_err(format!("encode message: {e}")))?;
    chan.send(&raw)
}

pub(super) fn recv_msg<S: Read + Write>(
    chan: &mut SecureChannel<S>,
) -> Result<Msg, SyncError> {
    let raw = chan.recv()?;
    serde_json::from_slice(&raw)
        .map_err(|e| proto_err(format!("decode message: {e}")))
}
