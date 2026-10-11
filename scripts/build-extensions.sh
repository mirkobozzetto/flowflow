#!/bin/bash
# Build the Share extension .appex (xcodebuild, unsigned - sign-widget.sh
# signs every appex afterwards) and drop it into the app's PlugIns. dx only
# knows widgetkit extensions, so this one is built by its own tiny project.
set -euo pipefail

MODE="${1:-debug}"
APP_DIR="target/dx/flowflow/${MODE}/ios/Flowflow.app"
PROJ="src/ios/share/ShareExt.xcodeproj"
DERIVED="target/share-ext-derived"
CONFIG="Release"

if [ ! -d "$APP_DIR" ]; then
    echo "[share-ext] No app at $APP_DIR, skipping"
    exit 0
fi

echo "[share-ext] Building ShareExt.appex..."
xcodebuild -quiet \
    -project "$PROJ" \
    -target ShareExt \
    -configuration "$CONFIG" \
    SYMROOT="$(pwd)/$DERIVED" \
    build

APPEX="$DERIVED/${CONFIG}-iphoneos/ShareExt.appex"
if [ ! -d "$APPEX" ]; then
    echo "[share-ext] ERROR: build produced no appex at $APPEX"
    exit 1
fi

mkdir -p "$APP_DIR/PlugIns"
rm -rf "$APP_DIR/PlugIns/ShareExt.appex"
cp -R "$APPEX" "$APP_DIR/PlugIns/ShareExt.appex"
echo "[share-ext] Copied into $APP_DIR/PlugIns/ShareExt.appex"
