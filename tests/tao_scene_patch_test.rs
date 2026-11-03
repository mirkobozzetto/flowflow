// tao 0.35.3 hands UIKit a scene configuration it has already released, so
// the App Store build crashes at launch. The vendored copy in patches/tao
// carries tao 0.37.1's one-line fix; these checks keep it wired in.
use std::fs;

const PATCH_DIR: &str = "patches/tao";
const PATCHED_VIEW: &str = "patches/tao/src/platform_impl/ios/view.rs";
const TAO_VERSION: &str = "0.35.3";
const PATCH_LINE: &str = "tao = { path = \"patches/tao\" }";
const FIXED_RETURN: &str = "Retained::autorelease_ptr(config) as _";
const DANGLING_RETURN: &str = "Retained::as_ptr(&config)";

#[test]
fn cargo_patches_tao_with_the_vendored_copy() {
    let manifest = fs::read_to_string("Cargo.toml").unwrap();
    let patch = manifest
        .split("[patch.crates-io]")
        .nth(1)
        .expect("Cargo.toml has no [patch.crates-io] section");
    assert!(
        patch.contains(PATCH_LINE),
        "tao is not patched to {PATCH_DIR}"
    );
}

#[test]
fn the_lockfile_resolves_tao_to_the_vendored_copy() {
    let lock = fs::read_to_string("Cargo.lock").unwrap();
    let tao = lock
        .split("[[package]]")
        .find(|p| p.contains("\nname = \"tao\"\n"))
        .expect("no tao in Cargo.lock");
    assert!(tao.contains(&format!("version = \"{TAO_VERSION}\"")));
    assert!(
        !tao.contains("source ="),
        "tao still comes from the registry"
    );
}

#[test]
fn the_scene_configuration_is_autoreleased_not_dangling() {
    let view = fs::read_to_string(PATCHED_VIEW).unwrap();
    let body = view
        .split("fn configuration_for_connecting_scene_session")
        .nth(1)
        .and_then(|b| b.split("fn handle_deep_link").next())
        .expect("configuration_for_connecting_scene_session not found");
    assert!(body.contains(FIXED_RETURN));
    assert!(!body.contains(DANGLING_RETURN));
}
