#!/bin/bash
set -e

APP_PATH="target/dx/flowflow/debug/ios/Flowflow.app"
XCASSETS="AppIcon.xcassets"

if [ ! -d "$APP_PATH" ]; then
  echo "ERROR: $APP_PATH not found. Run dx build first."
  exit 1
fi

echo ">> Compiling icon asset catalog..."
xcrun actool --compile "$APP_PATH" \
  --platform iphoneos \
  --minimum-deployment-target 16.0 \
  --app-icon AppIcon \
  --output-partial-info-plist /tmp/appicon-partial.plist \
  "$XCASSETS" 2>&1 || true

if [ ! -f /tmp/appicon-partial.plist ]; then
  echo "ERROR: actool failed to produce icon plist."
  exit 1
fi

echo ">> Merging icon into Info.plist..."
/usr/libexec/PlistBuddy -c "Delete :CFBundleIcons" "$APP_PATH/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Delete :CFBundleIcons~ipad" "$APP_PATH/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Merge /tmp/appicon-partial.plist" "$APP_PATH/Info.plist"

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

echo ">> Installing on device..."
DEVICE_ID=$(xcrun devicectl list devices 2>/dev/null | grep "available (paired)" | grep -oE '[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}')
if [ -z "$DEVICE_ID" ]; then
  echo "ERROR: No paired device found."
  exit 1
fi
xcrun devicectl device install app --device "$DEVICE_ID" "$APP_PATH" 2>&1

echo ">> Done."
