#!/bin/bash
set -euo pipefail

MODE="${1:-debug}"
APP_DIR="target/dx/flowflow/${MODE}/ios/Flowflow.app"
APPEX_DIR="${APP_DIR}/PlugIns/recording_widget.appex"
TEAM_ID="R477R8NK27"
WIDGET_BUNDLE_ID="com.mirkobozzetto.flowflow.recording-widget"

if [ "$MODE" = "release" ]; then
    SIGNING_ID=$(security find-identity -v -p codesigning | awk -F'"' '/Apple Distribution/{print $2; exit}')
    if [ -z "$SIGNING_ID" ]; then
        echo "[sign-widget] ERROR: no 'Apple Distribution' identity in Keychain"
        echo "[sign-widget] Create one at https://developer.apple.com/account/resources/certificates/list"
        exit 1
    fi
    GET_TASK_ALLOW="false"
    PROFILE_PATTERN="distribution"
else
    SIGNING_ID=$(security find-identity -v -p codesigning | awk -F'"' '/Apple Development/{print $2; exit}')
    if [ -z "$SIGNING_ID" ]; then
        echo "[sign-widget] ERROR: no 'Apple Development' identity in Keychain"
        exit 1
    fi
    GET_TASK_ALLOW="true"
    PROFILE_PATTERN="development"
fi

echo "[sign-widget] Mode: $MODE"
echo "[sign-widget] Signing identity: $SIGNING_ID"

if [ ! -d "$APPEX_DIR" ]; then
    echo "[sign-widget] No .appex found at $APPEX_DIR"
    exit 0
fi

WIDGET_PROFILE=""
for f in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision; do
    PROFILE_PLIST=$(security cms -D -i "$f" 2>/dev/null || true)
    if echo "$PROFILE_PLIST" | grep -q "$WIDGET_BUNDLE_ID"; then
        if [ "$MODE" = "release" ]; then
            if echo "$PROFILE_PLIST" | grep -q "ProvisionedDevices"; then
                continue
            fi
            WIDGET_PROFILE="$f"
            break
        else
            if echo "$PROFILE_PLIST" | grep -q "ProvisionedDevices"; then
                WIDGET_PROFILE="$f"
                break
            fi
        fi
    fi
done

if [ -z "$WIDGET_PROFILE" ]; then
    echo "[sign-widget] ERROR: no $PROFILE_PATTERN provisioning profile for $WIDGET_BUNDLE_ID"
    if [ "$MODE" = "release" ]; then
        echo "[sign-widget] Create an App Store provisioning profile for the widget at"
        echo "[sign-widget]   https://developer.apple.com/account/resources/profiles/add"
        echo "[sign-widget]   Type: App Store. App ID: $WIDGET_BUNDLE_ID. Certificate: Apple Distribution."
    fi
    exit 1
fi

echo "[sign-widget] Using profile: $WIDGET_PROFILE"

cp "$WIDGET_PROFILE" "${APPEX_DIR}/embedded.mobileprovision"

ENTITLEMENTS="target/widget-entitlements.plist"
rm -f "$ENTITLEMENTS"
cat > "$ENTITLEMENTS" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>application-identifier</key>
    <string>${TEAM_ID}.${WIDGET_BUNDLE_ID}</string>
    <key>get-task-allow</key>
    <${GET_TASK_ALLOW}/>
    <key>com.apple.developer.team-identifier</key>
    <string>${TEAM_ID}</string>
    <key>com.apple.security.application-groups</key>
    <array>
        <string>group.com.mirkobozzetto.flowflow</string>
    </array>
</dict>
</plist>
PLIST

ICON_SRC="assets/flowflow-icon-300.png"
if [ -f "$ICON_SRC" ]; then
    cp "$ICON_SRC" "${APPEX_DIR}/flowflow-icon.png"
fi

# App Intents metadata must land in the appex BEFORE the seal is applied.
bash scripts/gen-appintents-metadata.sh "$MODE"

codesign --force --entitlements "$ENTITLEMENTS" --sign "$SIGNING_ID" "$APPEX_DIR"
echo "[sign-widget] Signed $APPEX_DIR"

rm -f "$ENTITLEMENTS"

codesign --force --sign "$SIGNING_ID" --preserve-metadata=entitlements "$APP_DIR"
echo "[sign-widget] Re-signed main app"
