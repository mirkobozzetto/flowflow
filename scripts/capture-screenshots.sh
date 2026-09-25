#!/bin/bash
# App Store screenshots from the iOS simulator, one locale per run, no hands.
#
#   scripts/capture-screenshots.sh en /tmp/flowflow-demo-en.db
#
# Needs a debug simulator build (docs/release/) and a demo store from
# `cargo run --example demo_store`, so no personal note reaches a screenshot.
# Each screen is set up by the debug-only screenshot watcher
# (src/ui/app/watchers.rs), which reads a `shot` file next to the store.
# Output: screenshots/<version>/<lang>/NN.png.
set -euo pipefail

LANG_CODE="${1:?usage: $0 en|fr demo.db}"
DEMO_DB="${2:?usage: $0 en|fr demo.db}"
DEVICE="iPhone 17 Pro Max"
BUNDLE=com.mirkobozzetto.flowflow
APP=target/dx/flowflow/debug/ios/Flowflow.app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="screenshots/$VERSION/$LANG_CODE"
EXPECTED="1320x2868"

# App Store order: the first screenshot sells the app.
SHOTS=(home record note chat menu)

# Fresh demo store on every start: a take left running must not leak into
# the next screens.
reset() {
  xcrun simctl terminate booted "$BUNDLE" 2>/dev/null || true
  rm -f "$DOCS"/flowflow.db* "$DOCS/shot"
  cp "$DEMO_DB" "$DOCS/flowflow.db"
  xcrun simctl launch booted "$BUNDLE" >/dev/null
  sleep 10
}

# A clean boot: a pending system alert would sit on every screenshot.
xcrun simctl shutdown "$DEVICE" 2>/dev/null || true
xcrun simctl boot "$DEVICE"
xcrun simctl bootstatus "$DEVICE" -b >/dev/null
xcrun simctl terminate booted "$BUNDLE" 2>/dev/null || true
xcrun simctl install booted "$APP"
xcrun simctl privacy booted grant microphone "$BUNDLE"
DOCS="$(xcrun simctl get_app_container booted "$BUNDLE" data)/Documents"
mkdir -p "$DOCS"
reset
# Apple's marketing status bar: 9:41, full battery and signal.
xcrun simctl status_bar booted override --time 9:41 --batteryState charged \
  --batteryLevel 100 --cellularBars 4 --wifiBars 3

mkdir -p "$OUT"
for i in "${!SHOTS[@]}"; do
  n=$(printf '%02d' $((i + 1)))
  echo "${SHOTS[$i]}" > "$DOCS/shot"
  sleep 3
  xcrun simctl io booted screenshot "$OUT/$n.png" >/dev/null 2>&1
  size=$(sips -g pixelWidth -g pixelHeight "$OUT/$n.png" |
    awk '/pixelWidth/{w=$2} /pixelHeight/{h=$2} END{print w "x" h}')
  [ "$size" = "$EXPECTED" ] || echo "WARN $OUT/$n.png is $size, expected $EXPECTED"
  echo "$OUT/$n.png  ${SHOTS[$i]}"
  if [ "${SHOTS[$i]}" = record ]; then reset; fi
done

xcrun simctl status_bar booted clear
