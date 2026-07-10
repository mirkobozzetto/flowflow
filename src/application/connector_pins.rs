// Connector manifest pinning (RFC 0016): the device downloads the signed manifest envelopes it is
// entitled to, verifies each against the pinned admin key, and pins them locally - the same trust
// regime as agent packages. Resolution by connector TYPE takes the lowest pinned slug (the
// deterministic order the backend loader and console share, F4). Anti-rollback: a fetched manifest
// older than the pin is refused (F2). A revoked slug is force-unpinned even while an installed
// agent still requires it (F3); the agent then fails to arm with a clear message.

use serde_json::Value;

use crate::domain::agent_manifest::{verify_envelope, ADMIN_PUBKEY};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::persistence::installed_connector_repo::PinnedConnector;
use crate::infrastructure::persistence::Database;

// Interpret a VERIFIED manifest value into a pin row. Pure; separated from the network so tests
// drive it directly. `slug` is the CATALOG id from the envelope (what the proxy routes on, e.g.
// "google"), distinct from the manifest's own name ("google-sheets"). Version required, >= 1.
pub fn pin_from_manifest(
    manifest: &Value,
    digest: &str,
    slug: &str,
) -> Result<PinnedConnector, String> {
    if slug.is_empty() {
        return Err("envelope has no slug".into());
    }
    let connector_type = manifest
        .get("type")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("manifest has no type")?;
    let version = manifest
        .get("version")
        .and_then(Value::as_i64)
        .filter(|v| *v >= 1)
        .ok_or("manifest.version must be a positive integer")?;
    Ok(PinnedConnector {
        slug: slug.to_string(),
        connector_type: connector_type.to_string(),
        version,
        content_digest: digest.to_string(),
        manifest_json: serde_json::to_string(manifest)
            .map_err(|e| e.to_string())?,
    })
}

// Verify one envelope and pin it, honoring the anti-rollback gate. Returns the slug on success,
// None when skipped (stale version), Err on a verification/parse failure.
pub fn verify_and_pin(
    db: &Database,
    envelope: &Value,
) -> Result<Option<String>, String> {
    verify_and_pin_with(db, envelope, ADMIN_PUBKEY)
}

// Key-parameterized twin: production code pins against ADMIN_PUBKEY only; tests exercise the same
// path with the dev key (the prod private key never leaves the backend operator).
pub fn verify_and_pin_with(
    db: &Database,
    envelope: &Value,
    admin_pubkey: &str,
) -> Result<Option<String>, String> {
    let (manifest, digest) = verify_envelope(envelope, admin_pubkey)
        .map_err(|e| format!("envelope: {e}"))?;
    let slug = envelope
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let pin = pin_from_manifest(&manifest, &digest, slug)?;
    if let Some(existing) = db.pinned_connector(&pin.slug) {
        if pin.version < existing.version {
            eprintln!(
                "[connectors] refusing rollback for `{}`: fetched v{} < pinned v{}",
                pin.slug, pin.version, existing.version
            );
            return Ok(None);
        }
    }
    let slug = pin.slug.clone();
    db.pin_connector(&pin)?;
    Ok(Some(slug))
}

/// Refresh every entitled connector pin from the backend. Best-effort per envelope: one bad
/// envelope never blocks the others; failures are surfaced in the returned report string.
pub async fn refresh_pins(
    db: &Database,
    backend: &BackendClient,
) -> Result<usize, String> {
    let envelopes = backend
        .fetch_connector_manifests(db)
        .await
        .map_err(|e| format!("fetch manifests: {e}"))?;
    let mut pinned = 0;
    for env in &envelopes {
        match verify_and_pin(db, env) {
            Ok(Some(_)) => pinned += 1,
            Ok(None) => {}
            Err(e) => eprintln!("[connectors] envelope rejected: {e}"),
        }
    }
    Ok(pinned)
}

/// Apply the connector kill switch: a revoked slug loses its pin unconditionally (F3). An agent
/// that still requires it will fail to arm with a clear `no connector pinned` error instead of
/// dialing into silent 403s.
pub async fn apply_revocations(
    db: &Database,
    backend: &BackendClient,
) -> Result<usize, String> {
    let revoked = backend
        .fetch_connector_revocations(db)
        .await
        .map_err(|e| format!("fetch revocations: {e}"))?;
    let mut removed = 0;
    for r in &revoked {
        if db.pinned_connector(&r.slug).is_some() {
            db.remove_connector_pin(&r.slug)?;
            eprintln!("[connectors] revoked `{}`: pin removed", r.slug);
            removed += 1;
        }
    }
    Ok(removed)
}

/// The pinned (slug, manifest_json) for a required connector type - the data-driven replacement of
/// the old compiled `connector_for_type`. Local pins only; no network.
pub fn resolve_for_type(
    db: &Database,
    connector_type: &str,
) -> Option<(String, String)> {
    db.pinned_connector_for_type(connector_type)
        .map(|p| (p.slug, p.manifest_json))
}

/// Resolve a type, fetching + pinning from the backend when no local pin exists (the install-time
/// path). Falls back to the local-only resolution when the backend is unreachable.
pub async fn ensure_for_type(
    db: &Database,
    backend: Option<&BackendClient>,
    connector_type: &str,
) -> Option<(String, String)> {
    if let Some(found) = resolve_for_type(db, connector_type) {
        return Some(found);
    }
    if let Some(backend) = backend {
        if let Err(e) = refresh_pins(db, backend).await {
            eprintln!("[connectors] refresh failed: {e}");
        }
    }
    resolve_for_type(db, connector_type)
}
