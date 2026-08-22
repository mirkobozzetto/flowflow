use super::proto_err;
use crate::infrastructure::sync::transport::SecureChannel;
use crate::infrastructure::sync::SyncError;
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
        // Human-readable label for the pairing UI (v4). A label, never an
        // identity: authorization stays key-based.
        #[serde(default)]
        device_name: String,
        // Account-heal advertisement (RFC 0026). All defaulted: an older
        // build simply never runs the heal exchange (both Hellos must say
        // heal=true), no version bump needed. `premium` is self-declared
        // cache; the mint side re-verifies against its own backend anyway.
        #[serde(default)]
        heal: bool,
        #[serde(default)]
        premium: bool,
        #[serde(default)]
        backend_host: Option<String>,
    },
    RestoredFloor {
        floor: i64,
    },
    // Account-heal exchange (RFC 0026): symmetric and UNCONDITIONAL once
    // both Hellos advertised heal=true - a conditional frame on a blocking
    // stream would deadlock the side that expects nothing. Fields are None
    // when a side has nothing to ask or grant.
    JoinRequest {
        pubkey: Option<String>,
        // Ed25519 signature (base64) over the Noise handshake hash: binds
        // the enrolled pubkey to the peer of THIS session.
        sig: Option<String>,
    },
    JoinToken {
        token: Option<String>,
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
