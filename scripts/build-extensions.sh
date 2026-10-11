#!/bin/bash
# Build every app extension that has its own tiny project
# (src/ios/*/*.xcodeproj, target named after it; dx only knows widgetkit
# extensions), unsigned - sign-widget.sh signs every appex afterwards - and
# drop it into the app's PlugIns.
set -euo pipefail

MODE="${1:-debug}"
APP_DIR="target/dx/flowflow/${MODE}/ios/Flowflow.app"
CONFIG="Release"

if [ ! -d "$APP_DIR" ]; then
    echo "[extensions] No app at $APP_DIR, skipping"
    exit 0
fi

mkdir -p "$APP_DIR/PlugIns"
for PROJ in src/ios/*/*.xcodeproj; do
    NAME=$(basename "$PROJ" .xcodeproj)
    DERIVED="target/${NAME}-derived"
    echo "[extensions] Building $NAME.appex..."
    xcodebuild -quiet \
        -project "$PROJ" \
        -target "$NAME" \
        -configuration "$CONFIG" \
        SYMROOT="$(pwd)/$DERIVED" \
        build

    APPEX="$DERIVED/${CONFIG}-iphoneos/$NAME.appex"
    if [ ! -d "$APPEX" ]; then
        echo "[extensions] ERROR: build produced no appex at $APPEX"
        exit 1
    fi
    rm -rf "$APP_DIR/PlugIns/$NAME.appex"
    cp -R "$APPEX" "$APP_DIR/PlugIns/$NAME.appex"
    echo "[extensions] Copied into $APP_DIR/PlugIns/$NAME.appex"
done
