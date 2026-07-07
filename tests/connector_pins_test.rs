// RFC 0016 device half: signed connector manifests are verified against the pinned admin key and
// pinned locally. Pins: the v17 backfill, the envelope verify path (real dev-key signature), the
// anti-rollback gate, the deterministic type resolution (lowest slug), pin removal (revocation),
// and the drift guard tying the compiled fixture to the canonical marketplace JSON.

use flowflow::application::connector_pins::{
    pin_from_manifest, resolve_for_type, verify_and_pin_with,
};
use flowflow::domain::agent_manifest::digest_of;
use flowflow::infrastructure::persistence::installed_connector_repo::{
    PinnedConnector, SHEETS_BACKFILL_MANIFEST_JSON,
};
use flowflow::infrastructure::persistence::Database;
use serde_json::{json, Value};
use tempfile::tempdir;

// Digest + dev-key signature of the CANONICAL connectors/google-sheets.json (marketplace repo),
// computed offline with the dev admin seed - the same pair the backend serves in dev. If the
// canonical manifest changes, recompute both and update the device fixture in the same change.
const CANONICAL_SHEETS_DIGEST: &str =
    "sha256:8183f4a6e1e3dd599d50832689e397d46ef1bc8b56fc65ded762589aecb8baaf";
// The dev verification key matching the offline dev seed used to sign the test envelope.
const DEV_PUBKEY: &str = "ed25519:6kpsY+KcUgq+9VB7Ey7F+ZVHdq6+vnuSQh7qaRRG0iw=";
const CANONICAL_SHEETS_SIGNATURE: &str = "ed25519:J55l9QqLedgkSaBSgpeevP3WsHEKyT8CKDRii7PvrjGV6sdjuNDI2sQg8jNnx6TWQYwk/TXjzXXvbO4lE3H8Ag==";

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");
    (db, dir)
}

fn sheets_manifest() -> Value {
    serde_json::from_str(SHEETS_BACKFILL_MANIFEST_JSON).expect("fixture parses")
}

fn signed_sheets_envelope() -> Value {
    json!({
        "slug": "google",
        "manifest": sheets_manifest(),
        "content_digest": CANONICAL_SHEETS_DIGEST,
        "signature": CANONICAL_SHEETS_SIGNATURE,
        "signer_key_id": "dev-admin",
    })
}

#[test]
fn fixture_matches_canonical_digest() {
    // The drift guard (RFC 0016 F7): the compiled backfill fixture must stay byte-identical in
    // canonical form to marketplace-flowflow/connectors/google-sheets.json.
    assert_eq!(digest_of(&sheets_manifest()), CANONICAL_SHEETS_DIGEST);
}

#[test]
fn v17_backfills_sheets_pin() {
    let (db, _dir) = open_test_db();
    let pin = db.pinned_connector("google").expect("backfilled");
    assert_eq!(pin.connector_type, "tabular_store");
    assert_eq!(pin.version, 1);
    assert_eq!(pin.content_digest, CANONICAL_SHEETS_DIGEST);
}

#[test]
fn verified_envelope_pins() {
    let (db, _dir) = open_test_db();
    db.remove_connector_pin("google").unwrap();

    let slug = verify_and_pin_with(&db, &signed_sheets_envelope(), DEV_PUBKEY)
        .expect("envelope verifies")
        .expect("pinned, not skipped");
    assert_eq!(slug, "google");
    let pin = db.pinned_connector("google").unwrap();
    assert_eq!(pin.content_digest, CANONICAL_SHEETS_DIGEST);
}

#[test]
fn tampered_manifest_never_pins() {
    let (db, _dir) = open_test_db();
    db.remove_connector_pin("google").unwrap();

    let mut env = signed_sheets_envelope();
    // reclassify a destructive-capable write as read_only: the digest no longer matches
    env["manifest"]["tools"][5]["risk"] = json!("read_only");
    assert!(verify_and_pin_with(&db, &env, DEV_PUBKEY).is_err());
    assert!(db.pinned_connector("google").is_none(), "nothing pinned");
}

#[test]
fn rollback_is_refused() {
    let (db, _dir) = open_test_db();
    // simulate a newer pin than the served envelope (backend rollback / freeze)
    db.pin_connector(&PinnedConnector {
        slug: "google".into(),
        connector_type: "tabular_store".into(),
        version: 2,
        content_digest: "sha256:newer".into(),
        manifest_json: SHEETS_BACKFILL_MANIFEST_JSON.into(),
    })
    .unwrap();

    let skipped =
        verify_and_pin_with(&db, &signed_sheets_envelope(), DEV_PUBKEY)
            .expect("valid envelope");
    assert!(skipped.is_none(), "v1 over v2 must be skipped");
    assert_eq!(db.pinned_connector("google").unwrap().version, 2);
}

#[test]
fn type_resolution_takes_lowest_slug() {
    let (db, _dir) = open_test_db();
    // backfill pinned `google`; add a lexicographically smaller sibling of the same type
    db.pin_connector(&PinnedConnector {
        slug: "airtable".into(),
        connector_type: "tabular_store".into(),
        version: 1,
        content_digest: "sha256:x".into(),
        manifest_json: "{}".into(),
    })
    .unwrap();

    let (slug, _) = resolve_for_type(&db, "tabular_store").expect("resolves");
    assert_eq!(slug, "airtable", "lowest slug wins (shared order, F4)");
}

#[test]
fn removed_pin_disarms_resolution() {
    let (db, _dir) = open_test_db();
    db.remove_connector_pin("google").unwrap();
    assert!(resolve_for_type(&db, "tabular_store").is_none());
}

#[test]
fn manifest_without_version_is_rejected() {
    let mut m = sheets_manifest();
    m.as_object_mut().unwrap().remove("version");
    assert!(pin_from_manifest(&m, "sha256:x", "google").is_err());
}
