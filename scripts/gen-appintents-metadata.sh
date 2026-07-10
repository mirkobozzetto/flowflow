#!/bin/bash
# Emit Metadata.appintents into the widget .appex AND the app bundle. Xcode
# generates these at build time; our dx pipeline does not, and without them
# iOS shows Control Center buttons whose taps silently do nothing (the system
# resolves intents from this metadata, and requires the intent registered in
# BOTH bundles - "target membership: app + extension").
#
# Each bundle's metadata is generated from the sources of the module actually
# LINKED into that binary (widget: RecordingWidget; app: RecordingPlugin,
# statically linked into the Rust executable) so the mangled type names match
# a real runtime type - a copied foreign-module metadata resolves to nothing.
#
# Recipe (CodexBar PR #783 / Bazel rules_apple): build the SPM package once
# with xcodebuild to harvest .swiftconstvalues, then run Apple's
# appintentsmetadataprocessor over the sources in compile-time mode.
set -euo pipefail

MODE="${1:-debug}"
APP_DIR="target/dx/flowflow/${MODE}/ios/Flowflow.app"
APPEX_DIR="${APP_DIR}/PlugIns/recording_widget.appex"
DERIVED="target/appintents-derived"

if [ ! -d "$APPEX_DIR" ]; then
    echo "[appintents] No .appex at $APPEX_DIR, skipping"
    exit 0
fi

TOOLCHAIN="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain"
SDK=$(xcrun --sdk iphoneos --show-sdk-path)
XCODE_BUILD=$(xcodebuild -version | awk '/Build version/ {print $3}')

gen_metadata() {
    local pkg_dir="$1"
    local module="$2"
    local out_dir="$3"

    echo "[appintents] Building $module via xcodebuild (const-values harvest)..."
    (cd "$pkg_dir" && xcodebuild -quiet \
        -scheme "$module" \
        -configuration Release \
        -destination generic/platform=iOS \
        -derivedDataPath "../../../$DERIVED" \
        build)

    local const_list src_list out
    const_list="target/appintents-constvals-${module}.txt"
    rm -f "$const_list"
    find "$DERIVED/Build/Intermediates.noindex" -name '*.swiftconstvalues' \
        -path "*${module}*" > "$const_list"
    if [ ! -s "$const_list" ]; then
        echo "[appintents] ERROR: no .swiftconstvalues for $module"
        exit 1
    fi

    src_list="target/appintents-sources-${module}.txt"
    rm -f "$src_list"
    find "$(pwd)/$pkg_dir/Sources" -name '*.swift' > "$src_list"

    echo "[appintents] Processing $module -> $out_dir"
    out=$(xcrun appintentsmetadataprocessor \
        --output "$out_dir" \
        --toolchain-dir "$TOOLCHAIN" \
        --module-name "$module" \
        --sdk-root "$SDK" \
        --xcode-version "$XCODE_BUILD" \
        --platform-family iOS \
        --deployment-target 16.2 \
        --target-triple arm64-apple-ios16.2 \
        --source-file-list "$src_list" \
        --swift-const-vals-list "$const_list" \
        --compile-time-extraction 2>&1) || {
        echo "$out"
        exit 1
    }
    echo "$out"
    rm -f "$const_list" "$src_list"

    if echo "$out" | grep -qE "error:|skipping writing output"; then
        echo "[appintents] ERROR: processor did not extract intents ($module)"
        exit 1
    fi
    if [ ! -f "$out_dir/Metadata.appintents/extract.actionsdata" ]; then
        echo "[appintents] ERROR: Metadata.appintents missing ($module)"
        exit 1
    fi
}

gen_metadata "src/ios/widget" "RecordingWidget" "$APPEX_DIR"
gen_metadata "src/ios/plugin" "RecordingPlugin" "$APP_DIR"
echo "[appintents] Metadata.appintents written into appex + app bundle"
