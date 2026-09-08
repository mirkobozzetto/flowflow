#!/bin/bash
set -e

APP_PATH="${APP_PATH:-target/dx/flowflow/debug/ios/Flowflow.app}"
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
  "$XCASSETS" 2>&1

if [ ! -f /tmp/appicon-partial.plist ]; then
  echo "ERROR: actool failed to produce icon plist."
  exit 1
fi

echo ">> Merging icon into Info.plist..."
/usr/libexec/PlistBuddy -c "Delete :CFBundleIcons" "$APP_PATH/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Delete :CFBundleIcons~ipad" "$APP_PATH/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Merge /tmp/appicon-partial.plist" "$APP_PATH/Info.plist"

if [[ "$APP_PATH" == *"/release/"* ]]; then
  echo ">> Release build: icon merged into Info.plist; signing deferred to make appstore."
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

echo ">> Installing on device..."
if [ -z "${DEVICE_ID:-}" ]; then
  devices_json=$(mktemp)
  trap 'rm -f "$devices_json"' EXIT
  xcrun devicectl list devices --json-output "$devices_json"
  DEVICE_ID=$(python3 - "$devices_json" <<'PY'
import json
import sys

with open(sys.argv[1]) as source:
    devices = json.load(source)["result"]["devices"]
ids = [d["identifier"] for d in devices
       if d.get("connectionProperties", {}).get("pairingState") == "paired"
       and d.get("hardwareProperties", {}).get("platform") == "iOS"
       and d.get("hardwareProperties", {}).get("reality") == "physical"]
if len(ids) != 1:
    sys.exit("ERROR: Expected one paired iOS device; set DEVICE_ID explicitly.")
print(ids[0])
PY
  )
fi
xcrun devicectl device install app --device "$DEVICE_ID" "$APP_PATH" 2>&1

echo ">> Done."
