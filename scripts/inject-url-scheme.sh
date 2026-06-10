#!/bin/bash
set -e

APP_PATH="${APP_PATH:-target/dx/flowflow/debug/ios/Flowflow.app}"
URL_NAME="com.mirkobozzetto.flowflow"
URL_SCHEME="flowflow"

if [ ! -d "$APP_PATH" ]; then
  echo "ERROR: $APP_PATH not found. Run dx build first."
  exit 1
fi

PLIST="$APP_PATH/Info.plist"

echo ">> Registering $URL_SCHEME:// URL scheme in Info.plist..."

EXISTING=$(/usr/libexec/PlistBuddy -c "Print :CFBundleURLTypes" "$PLIST" 2>/dev/null || true)
if echo "$EXISTING" | grep -q "$URL_SCHEME"; then
  echo ">> Scheme already present; nothing to do."
else
  /usr/libexec/PlistBuddy -c "Delete :CFBundleURLTypes" "$PLIST" 2>/dev/null || true
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes array" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0 dict" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLName string $URL_NAME" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes array" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string $URL_SCHEME" "$PLIST"
fi

if [[ "$APP_PATH" == *"/release/"* ]]; then
  echo ">> Release build: scheme merged into Info.plist; signing deferred to make appstore."
  echo ">> Done."
  exit 0
fi

echo ">> Extracting entitlements from provisioning profile..."
security cms -D -i "$APP_PATH/embedded.mobileprovision" 2>/dev/null \
  | xmllint --xpath '//key[text()="Entitlements"]/following-sibling::dict[1]' - 2>/dev/null \
  > /tmp/ent-dict.plist
printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0">\n' > /tmp/ent.plist
cat /tmp/ent-dict.plist >> /tmp/ent.plist
printf '\n</plist>\n' >> /tmp/ent.plist

echo ">> Re-signing app..."
IDENTITY=$(security find-identity -v -p codesigning | grep "Apple Development" | head -1 | awk -F'"' '{print $2}')
codesign --force --sign "$IDENTITY" --entitlements /tmp/ent.plist "$APP_PATH"

echo ">> Done."
