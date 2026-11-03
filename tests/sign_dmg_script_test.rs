// scripts/sign-dmg.sh, run against fake `security` and `codesign`: no real
// keychain or certificate is touched.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const SCRIPT: &str = "scripts/sign-dmg.sh";
const IDENTITY: &str = "Developer ID Application: Demo Maker (TEAM123456)";

fn fake(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/bash\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Runs the script with `security` listing `identities`; returns whether it
/// succeeded and the arguments `codesign` got, if it was called.
fn run(identities: &str) -> (bool, Option<String>) {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let calls = tmp.path().join("codesign-args");
    fake(&bin, "security", &format!("printf '%s\\n' '{identities}'"));
    fake(
        &bin,
        "codesign",
        &format!("printf '%s\\n' \"$@\" > '{}'", calls.display()),
    );
    let dmg = tmp.path().join("FlowFlow-9.9.9-macos-arm64.dmg");
    fs::write(&dmg, b"dmg").unwrap();

    let status = Command::new("bash")
        .arg(SCRIPT)
        .arg(&dmg)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .status()
        .unwrap();
    let args = fs::read_to_string(&calls)
        .ok()
        .map(|a| a.replace(&dmg.display().to_string(), "<dmg>"));
    (status.success(), args)
}

#[test]
fn signs_the_dmg_with_the_developer_id_and_a_timestamp() {
    let listing = format!(
        "  1) ABCDEF \"Apple Development: Demo Maker (XYZ)\"\n  2) 123456 \"{IDENTITY}\"\n     2 valid identities found"
    );
    let (ok, args) = run(&listing);
    assert!(ok);
    let args = args.expect("codesign was not called");
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(
        args,
        ["--force", "--timestamp", "--sign", IDENTITY, "<dmg>"]
    );
}

#[test]
fn leaves_the_dmg_unsigned_without_a_developer_id() {
    let (ok, args) = run("  1) ABCDEF \"Apple Development: Demo Maker (XYZ)\"");
    assert!(ok);
    assert_eq!(args, None);
}

#[test]
fn make_dmg_signs_the_image_it_creates() {
    let makefile = fs::read_to_string("Makefile").unwrap();
    let recipe = makefile
        .split("\ndmg:")
        .nth(1)
        .and_then(|r| r.split("\n\n").next())
        .expect("no dmg target");
    let created = recipe.find("hdiutil create").expect("no hdiutil create");
    let signed = recipe
        .find("scripts/sign-dmg.sh dist/FlowFlow-$(VERSION)-macos-arm64.dmg")
        .expect("the dmg target does not sign the image");
    assert!(signed > created);
}
