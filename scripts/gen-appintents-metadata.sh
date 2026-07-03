#!/bin/bash
# Emit Metadata.appintents into the widget .appex. Xcode generates this file
# at build time; our dx pipeline does not, and without it iOS shows Control
# Center buttons whose taps silently do nothing (the system resolves intents
# from this metadata, never from the binary).
#
# Recipe (CodexBar PR #783 / Bazel rules_apple): build the SPM package once
# with xcodebuild to harvest .swiftconstvalues, then run Apple's
# appintentsmetadataprocessor over the sources in compile-time mode.
set -euo pipefail

MODE="${1:-debug}"
APP_DIR="target/dx/flowflow/${MODE}/ios/Flowflow.app"
APPEX_DIR="${APP_DIR}/PlugIns/recording_widget.appex"
PKG_DIR="src/ios/widget"
MODULE="RecordingWidget"
DERIVED="target/appintents-derived"

if [ ! -d "$APPEX_DIR" ]; then
    echo "[appintents] No .appex at $APPEX_DIR, skipping"
    exit 0
fi

TOOLCHAIN="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain"
SDK=$(xcrun --sdk iphoneos --show-sdk-path)
XCODE_BUILD=$(xcodebuild -version | awk '/Build version/ {print $3}')

echo "[appintents] Building $MODULE via xcodebuild (const-values harvest)..."
(cd "$PKG_DIR" && xcodebuild -quiet \
    -scheme "$MODULE" \
    -configuration Release \
    -destination generic/platform=iOS \
    -derivedDataPath "../../../$DERIVED" \
    build)

CONST_VALS_LIST=$(mktemp /tmp/appintents-constvals.XXXXX)
find "$DERIVED/Build/Intermediates.noindex" -name '*.swiftconstvalues' \
    -path "*${MODULE}*" > "$CONST_VALS_LIST"
if [ ! -s "$CONST_VALS_LIST" ]; then
    echo "[appintents] ERROR: no .swiftconstvalues produced"
    exit 1
fi

SRC_LIST=$(mktemp /tmp/appintents-sources.XXXXX)
find "$(pwd)/$PKG_DIR/Sources" -name '*.swift' > "$SRC_LIST"

echo "[appintents] Running appintentsmetadataprocessor..."
OUT=$(xcrun appintentsmetadataprocessor \
    --output "$APPEX_DIR" \
    --toolchain-dir "$TOOLCHAIN" \
    --module-name "$MODULE" \
    --sdk-root "$SDK" \
    --xcode-version "$XCODE_BUILD" \
    --platform-family iOS \
    --deployment-target 16.2 \
    --target-triple arm64-apple-ios16.2 \
    --source-file-list "$SRC_LIST" \
    --swift-const-vals-list "$CONST_VALS_LIST" \
    --compile-time-extraction 2>&1) || {
    echo "$OUT"
    exit 1
}
echo "$OUT"
rm -f "$CONST_VALS_LIST" "$SRC_LIST"

if echo "$OUT" | grep -qE "error:|skipping writing output"; then
    echo "[appintents] ERROR: processor did not extract intents"
    exit 1
fi
if [ ! -f "$APPEX_DIR/Metadata.appintents/extract.actionsdata" ]; then
    echo "[appintents] ERROR: Metadata.appintents missing after processing"
    exit 1
fi

# Apple requires the intent registered in BOTH bundles ("target membership:
# app + extension"); without the app-side copy the control tap is silently
# refused (LNActionExecutorErrorDomain 2018). Same metadata, app bundle root.
cp -R "$APPEX_DIR/Metadata.appintents" "$APP_DIR/Metadata.appintents"
echo "[appintents] Metadata.appintents written into appex + app bundle"
