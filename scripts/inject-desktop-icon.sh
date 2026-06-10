#!/bin/bash
set -e

APP_PATH="${APP_PATH:-target/dx/flowflow/debug/macos/Flowflow.app}"
SRC="AppIcon.xcassets/AppIcon.appiconset/icon-1024.png"

if [ ! -d "$APP_PATH" ]; then
  echo "ERROR: $APP_PATH not found. Run dx build first."
  exit 1
fi
if [ ! -f "$SRC" ]; then
  echo "ERROR: $SRC not found."
  exit 1
fi

echo ">> Building AppIcon.icns from $SRC..."
TMP=$(mktemp -d)
ICONSET="$TMP/AppIcon.iconset"
mkdir -p "$ICONSET"

sips -z 16 16     "$SRC" --out "$ICONSET/icon_16x16.png" >/dev/null
sips -z 32 32     "$SRC" --out "$ICONSET/icon_16x16@2x.png" >/dev/null
sips -z 32 32     "$SRC" --out "$ICONSET/icon_32x32.png" >/dev/null
sips -z 64 64     "$SRC" --out "$ICONSET/icon_32x32@2x.png" >/dev/null
sips -z 128 128   "$SRC" --out "$ICONSET/icon_128x128.png" >/dev/null
sips -z 256 256   "$SRC" --out "$ICONSET/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$SRC" --out "$ICONSET/icon_256x256.png" >/dev/null
sips -z 512 512   "$SRC" --out "$ICONSET/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$SRC" --out "$ICONSET/icon_512x512.png" >/dev/null
cp "$SRC" "$ICONSET/icon_512x512@2x.png"

iconutil -c icns "$ICONSET" -o "$APP_PATH/Contents/Resources/icon.icns"

PLIST="$APP_PATH/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleName FlowFlow" "$PLIST" 2>/dev/null || /usr/libexec/PlistBuddy -c "Add :CFBundleName string FlowFlow" "$PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName FlowFlow" "$PLIST" 2>/dev/null || /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string FlowFlow" "$PLIST"

touch "$APP_PATH"
echo ">> Desktop icon and branding injected into $APP_PATH"
