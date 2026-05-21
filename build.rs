use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();

    let (swift_triple, sdk_name) = match target.as_str() {
        "aarch64-apple-ios" => ("arm64-apple-ios", "iphoneos"),
        "aarch64-apple-ios-sim" => {
            ("arm64-apple-ios-simulator", "iphonesimulator")
        }
        "x86_64-apple-ios" => ("x86_64-apple-ios-simulator", "iphonesimulator"),
        _ => return,
    };

    let sdk_path = String::from_utf8(
        Command::new("xcrun")
            .args(["--sdk", sdk_name, "--show-sdk-path"])
            .output()
            .expect("xcrun --show-sdk-path failed")
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let package_path = manifest_dir.join("src/ios/plugin");

    let swift_target = match target.as_str() {
        "aarch64-apple-ios" => "arm64-apple-ios16.2",
        "aarch64-apple-ios-sim" => "arm64-apple-ios16.2-simulator",
        "x86_64-apple-ios" => "x86_64-apple-ios16.2-simulator",
        _ => unreachable!(),
    };

    let status = Command::new("xcrun")
        .args([
            "swift",
            "build",
            "--package-path",
            package_path.to_str().unwrap(),
            "--configuration",
            "release",
            "--triple",
            swift_triple,
            "--sdk",
            &sdk_path,
            "--product",
            "RecordingPlugin",
            "-Xswiftc",
            "-target",
            "-Xswiftc",
            swift_target,
        ])
        .status()
        .expect("xcrun swift build failed to start");

    if !status.success() {
        panic!("xcrun swift build failed with status: {status}");
    }

    let lib_dir = package_path
        .join(".build")
        .join(swift_triple)
        .join("release");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=RecordingPlugin");
    println!("cargo:rustc-link-lib=framework=ActivityKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rerun-if-changed=src/ios/plugin/Sources/");
}
