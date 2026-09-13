// Locks the trust anchor: a tampered manifest or a signature off the pinned admin key must NOT pin.
// `gen_fixture` (ignored) regenerates the dev key + the shipped fixture's digest/signature.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::Value;

use flowflow::application::connector_module::FIXTURE_PACKAGE;
use flowflow::domain::agent_manifest::{
    canonical_json, digest_of, verify_package, AgentManifestError, ADMIN_PUBKEY,
};

// The signer behind the shipped fixture. The PRODUCTION signer's public half is pinned as
// `ADMIN_PUBKEY`; its private seed never lives in the repo. Pass it via the environment when
// regenerating the fixture; absent, it falls back to the legacy [7u8;32] dev seed.
fn fixture_signing_key() -> SigningKey {
    if let Ok(h) = std::env::var("FIXTURE_SIGN_SEED") {
        let h = h.trim();
        if h.len() == 64 {
            let mut seed = [0u8; 32];
            for i in 0..32 {
                seed[i] = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16)
                    .expect("FIXTURE_SIGN_SEED must be 64 hex chars");
            }
            return SigningKey::from_bytes(&seed);
        }
    }
    SigningKey::from_bytes(&[7u8; 32])
}

// Prints the pinned key + the fixture's digest and signature. Run after editing the fixture manifest:
//   FIXTURE_SIGN_SEED=<64-hex prod seed> cargo test --test agent_manifest_test gen_fixture -- --ignored --nocapture
// then paste ADMIN_PUBKEY into domain/agent_manifest.rs and the digest/signature into the fixture.
#[test]
#[ignore]
fn gen_fixture() {
    let sk = fixture_signing_key();
    let pubkey = B64.encode(sk.verifying_key().to_bytes());

    let pkg: Value = serde_json::from_str(FIXTURE_PACKAGE).unwrap();
    let manifest = pkg.get("manifest").expect("fixture has a manifest");
    let digest = digest_of(manifest);
    let signature = B64.encode(sk.sign(digest.as_bytes()).to_bytes());

    println!("ADMIN_PUBKEY    = ed25519:{pubkey}");
    println!("content_digest  = {digest}");
    println!("signature       = ed25519:{signature}");
}

#[test]
fn fixture_verifies_against_pinned_key() {
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY)
        .expect("shipped fixture verifies against the pinned admin key");
    assert_eq!(verified.manifest.id, "agent-crm-sync");
    assert!(verified.content_digest.starts_with("sha256:"));
    // The stored canonical form must hash back to the pinned digest.
    let v: Value = serde_json::from_str(&verified.manifest_json).unwrap();
    assert_eq!(digest_of(&v), verified.content_digest);
}

#[test]
fn tampered_manifest_is_rejected() {
    // Flip a manifest field but keep the original digest + signature: the recomputed digest no longer
    // matches, so it fails before the signature is even checked.
    let tampered = FIXTURE_PACKAGE.replace("\"CRM Sync\"", "\"Evil Sync\"");
    assert_ne!(tampered, FIXTURE_PACKAGE);
    match verify_package(&tampered, ADMIN_PUBKEY) {
        Err(AgentManifestError::DigestMismatch { .. }) => {}
        other => panic!("expected DigestMismatch, got {other:?}"),
    }
}

#[test]
fn signature_off_a_different_key_is_rejected() {
    // The manifest is intact (digest matches), but we verify against a key the admin never signed with.
    let other = SigningKey::from_bytes(&[9u8; 32]);
    let other_pubkey =
        format!("ed25519:{}", B64.encode(other.verifying_key().to_bytes()));
    match verify_package(FIXTURE_PACKAGE, &other_pubkey) {
        Err(AgentManifestError::BadSignature) => {}
        other => panic!("expected BadSignature, got {other:?}"),
    }
}

#[test]
fn digest_is_canonical_and_key_order_independent() {
    let a: Value =
        serde_json::from_str(r#"{"b":1,"a":{"y":2,"x":3}}"#).unwrap();
    let b: Value =
        serde_json::from_str(r#"{"a":{"x":3,"y":2},"b":1}"#).unwrap();
    assert_eq!(canonical_json(&a), canonical_json(&b));
    assert_eq!(digest_of(&a), digest_of(&b));
}

#[test]
fn unsupported_signed_schema_is_rejected_after_integrity_checks() {
    let sk = SigningKey::from_bytes(&[11u8; 32]);
    let key = format!("ed25519:{}", B64.encode(sk.verifying_key().to_bytes()));
    for version in ["2", "999", "", "01"] {
        let mut pkg: Value = serde_json::from_str(FIXTURE_PACKAGE).unwrap();
        pkg["manifest"]["schema_version"] = version.into();
        let digest = digest_of(&pkg["manifest"]);
        pkg["content_digest"] = digest.clone().into();
        pkg["signature"] = format!(
            "ed25519:{}",
            B64.encode(sk.sign(digest.as_bytes()).to_bytes())
        )
        .into();
        assert!(
            matches!(verify_package(&pkg.to_string(), &key), Err(AgentManifestError::UnsupportedSchemaVersion(v)) if v == version)
        );
    }
}

#[test]
fn stored_unknown_schema_cannot_load_or_build() {
    use flowflow::domain::agent_manifest::{parse_manifest, AgentManifest};
    let mut pkg: Value = serde_json::from_str(FIXTURE_PACKAGE).unwrap();
    pkg["manifest"]["schema_version"] = "2".into();
    let raw = pkg["manifest"].to_string();
    assert!(matches!(
        parse_manifest(&raw),
        Err(AgentManifestError::UnsupportedSchemaVersion(_))
    ));
    // Direct deserialization must not bypass the execution check.
    let manifest: AgentManifest = serde_json::from_str(&raw).unwrap();
    let error =
        flowflow::application::agent_builder::build_agent_multi(&manifest, &[])
            .err()
            .unwrap();
    assert!(error.contains("unsupported agent schema version"));
}

#[test]
fn legacy_schema_retains_unknown_metadata_and_digest() {
    use flowflow::domain::agent_manifest::parse_manifest;
    let pkg: Value = serde_json::from_str(FIXTURE_PACKAGE).unwrap();
    let raw = canonical_json(&pkg["manifest"]);
    assert_eq!(parse_manifest(&raw).unwrap().schema_version, "1");
    assert_eq!(
        digest_of(&pkg["manifest"]),
        pkg["content_digest"].as_str().unwrap()
    );
    let mut extended = pkg["manifest"].clone();
    extended["display_metadata"] =
        serde_json::json!({"future_label":"retained"});
    assert!(parse_manifest(&extended.to_string()).is_ok());
    assert_eq!(
        serde_json::from_str::<Value>(&canonical_json(&extended)).unwrap(),
        extended
    );
}
