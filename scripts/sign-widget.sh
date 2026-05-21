#!/bin/bash
set -euo pipefail

MODE="${1:-debug}"
APP_DIR="target/dx/flowflow/${MODE}/ios/Flowflow.app"
APPEX_DIR="${APP_DIR}/PlugIns/recording_widget.appex"
SIGNING_ID="Apple Development: mirko@mirko.re (3YL4GA2Y23)"
TEAM_ID="R477R8NK27"
WIDGET_BUNDLE_ID="com.mirkobozzetto.flowflow.recording-widget"

if [ ! -d "$APPEX_DIR" ]; then
    echo "[sign-widget] No .appex found at $APPEX_DIR"
    exit 0
fi

WIDGET_PROFILE=""
for f in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision; do
    if security cms -D -i "$f" 2>/dev/null | grep -q "$WIDGET_BUNDLE_ID"; then
        WIDGET_PROFILE="$f"
        break
    fi
done

if [ -z "$WIDGET_PROFILE" ]; then
    echo "[sign-widget] ERROR: no provisioning profile for $WIDGET_BUNDLE_ID"
    echo "[sign-widget] Create one in Xcode (Widget Extension target) or Apple Developer portal"
    exit 1
fi

echo "[sign-widget] Using profile: $WIDGET_PROFILE"

cp "$WIDGET_PROFILE" "${APPEX_DIR}/embedded.mobileprovision"

ENTITLEMENTS=$(mktemp /tmp/widget-entitlements.XXXXX.plist)
cat > "$ENTITLEMENTS" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>application-identifier</key>
    <string>${TEAM_ID}.${WIDGET_BUNDLE_ID}</string>
    <key>get-task-allow</key>
    <true/>
    <key>com.apple.developer.team-identifier</key>
    <string>${TEAM_ID}</string>
</dict>
</plist>
PLIST

ICON_SRC="assets/flowflow-icon-300.png"
if [ -f "$ICON_SRC" ]; then
    cp "$ICON_SRC" "${APPEX_DIR}/flowflow-icon.png"
fi

codesign --force --entitlements "$ENTITLEMENTS" --sign "$SIGNING_ID" "$APPEX_DIR"
echo "[sign-widget] Signed $APPEX_DIR"

rm -f "$ENTITLEMENTS"

codesign --force --sign "$SIGNING_ID" --preserve-metadata=entitlements "$APP_DIR"
echo "[sign-widget] Re-signed main app"
