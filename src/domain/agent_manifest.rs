use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::governance::Governance;
use crate::domain::orchestration::Orchestration;

// The on-device trust anchor (RFC 0010 docs/protocol/02). The admin holds the matching Ed25519
// private key; the device ships only this public key and verifies every package's signature against
// it before pinning. Swapping the real backend admin key later is a one-line change here.
// Dev key (deterministic seed [7u8; 32]); never the production signer.
pub const ADMIN_PUBKEY: &str =
    "ed25519:6kpsY+KcUgq+9VB7Ey7F+ZVHdq6+vnuSQh7qaRRG0iw=";

// The agent's behavior contract: model + governance (Layer 1) + orchestration (Layer 2) + advisory
// prompt. Unknown fields are tolerated so a newer manifest still parses for digest purposes; the
// digest is taken over the raw JSON, not this struct, so dropped fields never change integrity.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentManifest {
    pub schema_version: String,
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub alias: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub required_connectors: Vec<RequiredConnector>,
    #[serde(default)]
    pub system_prompt: String,
    pub governance: Governance,
    #[serde(default)]
    pub orchestration: Orchestration,
}

// Capability-style and connector-agnostic: "a tabular store with read/update", not "Google Sheets".
// The device binds it to whatever connector is armed (05).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequiredConnector {
    #[serde(rename = "type")]
    pub connector_type: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

// A verified, ready-to-pin agent. `manifest_json` is the canonical form the `content_digest` was
// computed over, so re-pinning and later integrity re-checks recompute the same digest.
#[derive(Debug, Clone)]
pub struct VerifiedAgent {
    pub manifest: AgentManifest,
    pub content_digest: String,
    pub manifest_json: String,
}

// One pinned row of `installed_agents`. Enforcement reads `manifest_json` as data only.
#[derive(Debug, Clone)]
pub struct InstalledAgent {
    pub id: String,
    pub version: String,
    pub content_digest: String,
    pub manifest_json: String,
    pub installed_at: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentManifestError {
    #[error("package json: {0}")]
    Parse(String),
    #[error("package is missing field `{0}`")]
    MissingField(&'static str),
    #[error("content_digest mismatch: package claims {expected}, manifest hashes to {got}")]
    DigestMismatch { expected: String, got: String },
    #[error("admin public key is malformed")]
    BadAdminKey,
    #[error("signature is malformed")]
    BadSignatureFormat,
    #[error("signature does not verify against the pinned admin key")]
    BadSignature,
}

// Recursively rebuild every object with keys in sorted order, then serialize compact. The signer and
// the device must hash the SAME bytes; doing the sort ourselves keeps the digest stable even if a
// dependency turns on serde_json's `preserve_order` (which would otherwise emit insertion order).
fn canonicalize(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for k in keys {
                sorted.insert(k.clone(), canonicalize(&map[k]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(canonicalize).collect())
        }
        other => other.clone(),
    }
}

pub fn canonical_json(v: &Value) -> String {
    serde_json::to_string(&canonicalize(v))
        .expect("a serde_json::Value always re-serializes")
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// `sha256:<hex>` over the canonical JSON of a manifest value. The digest the package signs.
pub fn digest_of(manifest: &Value) -> String {
    let mut h = Sha256::new();
    h.update(canonical_json(manifest).as_bytes());
    format!("sha256:{}", hex_lower(&h.finalize()))
}

fn strip(prefix: &str, s: &str) -> String {
    s.strip_prefix(prefix).unwrap_or(s).to_string()
}

fn parse_verifying_key(b64: &str) -> Result<VerifyingKey, AgentManifestError> {
    let raw = B64
        .decode(strip("ed25519:", b64))
        .map_err(|_| AgentManifestError::BadAdminKey)?;
    let bytes: [u8; 32] = raw
        .try_into()
        .map_err(|_| AgentManifestError::BadAdminKey)?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|_| AgentManifestError::BadAdminKey)
}

fn parse_signature(b64: &str) -> Result<Signature, AgentManifestError> {
    let raw = B64
        .decode(strip("ed25519:", b64))
        .map_err(|_| AgentManifestError::BadSignatureFormat)?;
    let bytes: [u8; 64] = raw
        .try_into()
        .map_err(|_| AgentManifestError::BadSignatureFormat)?;
    Ok(Signature::from_bytes(&bytes))
}

/// Verify an `AgentPackage` JSON: the manifest hashes to the claimed `content_digest`, and the admin
/// signed that digest. Both checks must pass before a package is pinned (02, 09). The digest is
/// recomputed from the package's own manifest bytes, so a tampered manifest fails even if its
/// signature is left intact.
pub fn verify_package(
    package_json: &str,
    admin_pubkey_b64: &str,
) -> Result<VerifiedAgent, AgentManifestError> {
    let pkg: Value = serde_json::from_str(package_json)
        .map_err(|e| AgentManifestError::Parse(e.to_string()))?;

    let manifest_value = pkg
        .get("manifest")
        .ok_or(AgentManifestError::MissingField("manifest"))?;
    let claimed_digest = pkg
        .get("content_digest")
        .and_then(Value::as_str)
        .ok_or(AgentManifestError::MissingField("content_digest"))?;
    let signature = pkg
        .get("signature")
        .and_then(Value::as_str)
        .ok_or(AgentManifestError::MissingField("signature"))?;

    let computed = digest_of(manifest_value);
    if computed != claimed_digest {
        return Err(AgentManifestError::DigestMismatch {
            expected: claimed_digest.to_string(),
            got: computed,
        });
    }

    let admin_key = parse_verifying_key(admin_pubkey_b64)?;
    let sig = parse_signature(signature)?;
    admin_key
        .verify_strict(claimed_digest.as_bytes(), &sig)
        .map_err(|_| AgentManifestError::BadSignature)?;

    let manifest: AgentManifest =
        serde_json::from_value(manifest_value.clone())
            .map_err(|e| AgentManifestError::Parse(e.to_string()))?;

    Ok(VerifiedAgent {
        manifest,
        content_digest: computed,
        manifest_json: canonical_json(manifest_value),
    })
}

/// Re-derive the digest of a stored canonical `manifest_json` (the integrity re-check at load time).
pub fn digest_of_stored(
    manifest_json: &str,
) -> Result<String, AgentManifestError> {
    let v: Value = serde_json::from_str(manifest_json)
        .map_err(|e| AgentManifestError::Parse(e.to_string()))?;
    Ok(digest_of(&v))
}

pub fn parse_manifest(
    manifest_json: &str,
) -> Result<AgentManifest, AgentManifestError> {
    serde_json::from_str(manifest_json)
        .map_err(|e| AgentManifestError::Parse(e.to_string()))
}
