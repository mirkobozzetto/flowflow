#!/bin/bash
set -euo pipefail

PROFILES_DIR="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
APP_BUNDLE_ID="${APP_BUNDLE_ID:-com.mirkobozzetto.flowflow}"
WIDGET_BUNDLE_ID="${WIDGET_BUNDLE_ID:-com.mirkobozzetto.flowflow.recording-widget}"
THRESHOLD_SEC="${RENEW_THRESHOLD_SEC:-86400}"

NOW_TS=$(date +%s)
CRITICAL=0

profile_expiry_for() {
    local target_bundle="$1"
    local team_id_app="R477R8NK27.$target_bundle"
    for f in "$PROFILES_DIR"/*.mobileprovision; do
        [ -f "$f" ] || continue
        local plist
        plist=$(security cms -D -i "$f" 2>/dev/null) || continue
        local app_id
        app_id=$(echo "$plist" | plutil -extract 'Entitlements.application-identifier' raw - 2>/dev/null || echo "")
        if [ "$app_id" = "$team_id_app" ]; then
            local exp
            exp=$(echo "$plist" | plutil -extract ExpirationDate raw - 2>/dev/null)
            local uuid
            uuid=$(echo "$plist" | plutil -extract UUID raw - 2>/dev/null)
            echo "$uuid|$exp"
            return 0
        fi
    done
    echo "|"
}

report_one() {
    local label="$1"
    local bundle="$2"
    local info uuid exp
    info=$(profile_expiry_for "$bundle")
    uuid="${info%%|*}"
    exp="${info##*|}"
    if [ -z "$uuid" ]; then
        printf "  %-7s %-50s MISSING\n" "$label" "$bundle"
        CRITICAL=1
        return
    fi
    local exp_ts
    exp_ts=$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$exp" +%s 2>/dev/null || echo 0)
    local remain=$((exp_ts - NOW_TS))
    local state
    if [ "$remain" -le 0 ]; then
        state="EXPIRED"
        CRITICAL=1
    elif [ "$remain" -lt "$THRESHOLD_SEC" ]; then
        state="EXPIRES_SOON"
        CRITICAL=1
    else
        local days=$((remain / 86400))
        state="OK (${days}d left)"
    fi
    printf "  %-7s %-50s %s | %s\n" "$label" "$bundle" "$exp" "$state"
}

echo "Provisioning profiles:"
report_one "app"    "$APP_BUNDLE_ID"
report_one "widget" "$WIDGET_BUNDLE_ID"

if [ "$CRITICAL" -eq 1 ]; then
    echo "[check-profiles] CRITICAL: at least one profile expired or expiring < $((THRESHOLD_SEC / 3600))h"
    exit 1
fi

echo "[check-profiles] OK"
exit 0
