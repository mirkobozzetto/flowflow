#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TEMPLATE_DIR="$REPO_ROOT/tools/provision-renew/template"
PROFILES_DIR="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
TEAM_ID="${TEAM_ID:-R477R8NK27}"
APP_BUNDLE_ID="${APP_BUNDLE_ID:-com.mirkobozzetto.flowflow}"
WIDGET_BUNDLE_ID="${WIDGET_BUNDLE_ID:-com.mirkobozzetto.flowflow.recording-widget}"

if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "[renew] ERROR: template missing at $TEMPLATE_DIR"
    exit 1
fi

renew_one() {
    local bundle="$1"
    local label="$2"
    local work_dir="/tmp/flowflow-renew-$(echo "$bundle" | tr '.' '-')"

    echo ""
    echo "[renew] === $label : $bundle ==="

    rm -rf "$work_dir"
    cp -R "$TEMPLATE_DIR" "$work_dir"

    local pbxproj="$work_dir/RenewApp.xcodeproj/project.pbxproj"
    sed -i '' "s|__BUNDLE_ID__|$bundle|g" "$pbxproj"
    sed -i '' "s|__TEAM_ID__|$TEAM_ID|g" "$pbxproj"

    cd "$work_dir"
    xcodebuild build \
        -project RenewApp.xcodeproj \
        -target RenewApp \
        -configuration Debug \
        -destination 'generic/platform=iOS' \
        -allowProvisioningUpdates \
        -allowProvisioningDeviceRegistration \
        CODE_SIGN_STYLE=Automatic \
        DEVELOPMENT_TEAM="$TEAM_ID" \
        PRODUCT_BUNDLE_IDENTIFIER="$bundle" \
        2>&1 | tail -20
    cd - > /dev/null

    rm -rf "$work_dir"
    echo "[renew] $label done"
}

mkdir -p "$PROFILES_DIR"

renew_one "$APP_BUNDLE_ID"    "app"
renew_one "$WIDGET_BUNDLE_ID" "widget"

echo ""
echo "[renew] Verifying:"
bash "$(dirname "$0")/check-profiles.sh"
