#!/bin/bash
# App Store screenshots and previews from the iOS simulator, one locale per
# run.
#
#   scripts/capture-screenshots.sh en /tmp/flowflow-demo-en.db
#
# Needs a debug simulator build (docs/release/) and a demo store from
# `cargo run --example demo_store`; this script serves its fake Hermes, so no
# personal note, Hermes address or skill reaches a screenshot.
# Each screen is set up by the debug-only screenshot watcher
# (src/ui/app/watchers.rs), which reads a `shot` file next to the store.
# No hands: the native "+" menu cannot open headless, so the skills show
# through "/" in Hermes' field. Output: screenshots/<version>/<lang>/NN.png
# and preview-*.mp4 (15 to 30 s, 886 x 1920, H.264, silent stereo AAC).
set -euo pipefail

LANG_CODE="${1:?usage: $0 en|fr demo.db}"
DEMO_DB="${2:?usage: $0 en|fr demo.db}"
DEVICE="iPhone 17 Pro Max"
BUNDLE=com.mirkobozzetto.flowflow
APP=target/dx/flowflow/debug/ios/Flowflow.app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="screenshots/$VERSION/$LANG_CODE"
EXPECTED="1320x2868"
# The simulator records frames only when the screen moves: the last one is
# held this long, so a preview lasts 15 to 30 s.
HOLD=8

case "$LANG_CODE" in
  fr)
    PHOTO=tableau.jpg
    QUESTION="Que reste-t-il à faire ?"
    VOICE=Thomas
    DICTATION="Appeler la banque demain à dix heures pour le prêt du studio."
    ASK="Prépare la facture pour Nordwind"
    NEW="Résume ma semaine"
    ;;
  *)
    PHOTO=whiteboard.jpg
    QUESTION="What's left before launch?"
    VOICE=Samantha
    DICTATION="Call the bank tomorrow at ten about the studio loan."
    ASK="Draft the invoice for Nordwind"
    NEW="Summarize my week"
    ;;
esac

# App Store order (docs/release/<version>.md): the first screenshot sells the
# app. Lines after a screen are the watcher's arguments.
SHOTS=(
  "hermes
photo=$PHOTO"
  "hermes
text=/"
  "hermes
text=$ASK"
  "hermes-new
text=$NEW"
  record
  note
  chat
  menu
)

cargo build -q --example demo_store
target/debug/examples/demo_store hermes "$LANG_CODE" >/dev/null &
HERMES_PID=$!
trap 'kill $HERMES_PID 2>/dev/null || true' EXIT

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
cp "examples/demo_store/whiteboard-$LANG_CODE.jpg" "$DOCS/$PHOTO"
reset
# Apple's marketing status bar: 9:41, full battery and signal.
xcrun simctl status_bar booted override --time 9:41 --batteryState charged \
  --batteryLevel 100 --cellularBars 4 --wifiBars 3

mkdir -p "$OUT"
rm -f "$OUT"/*.png "$OUT"/*.mp4
for i in "${!SHOTS[@]}"; do
  n=$(printf '%02d' $((i + 1)))
  screen="${SHOTS[$i]%%$'\n'*}"
  printf '%s\n' "${SHOTS[$i]}" > "$DOCS/shot"
  sleep 4
  xcrun simctl io booted screenshot "$OUT/$n.png" >/dev/null 2>&1
  size=$(sips -g pixelWidth -g pixelHeight "$OUT/$n.png" |
    awk '/pixelWidth/{w=$2} /pixelHeight/{h=$2} END{print w "x" h}')
  [ "$size" = "$EXPECTED" ] || echo "WARN $OUT/$n.png is $size, expected $EXPECTED"
  echo "$OUT/$n.png  $screen"
  if [ "$screen" = record ]; then reset; fi
done

# A preview plays shots on a timeline: "seconds|shot", one line per step; a
# "!" shot runs a command instead (the Mac speaks into the simulator's mic).
preview() {
  local name=$1 rec
  shift
  reset
  xcrun simctl io booted recordVideo --codec=h264 --force "$OUT/raw-$name.mov" 2>/dev/null &
  rec=$!
  sleep 1
  for step in "$@"; do
    case "${step#*|}" in
      !*) eval "${step#*|!}" & ;;
      *) printf '%s\n' "${step#*|}" > "$DOCS/shot" ;;
    esac
    sleep "${step%%|*}"
  done
  kill -INT "$rec"
  wait "$rec" || true
  ffmpeg -y -loglevel error -i "$OUT/raw-$name.mov" \
    -f lavfi -i anullsrc=channel_layout=stereo:sample_rate=44100 \
    -vf "scale=886:-2,crop=886:1920,fps=30,tpad=stop_mode=clone:stop_duration=$HOLD" \
    -c:v libx264 -profile:v high -level 4.0 -pix_fmt yuv420p -c:a aac -shortest \
    "$OUT/preview-$name.mp4"
  rm -f "$OUT/raw-$name.mov"
  echo "$OUT/preview-$name.mp4"
}

preview dictate "2|home" "1|record" "6|!say -v $VOICE \"$DICTATION\"" "12|tap
selector=.voice-capsule button:last-of-type"
preview hermes "4|hermes-new
photo=$PHOTO
text=$QUESTION" "13|tap
selector=.composer-act-send"

xcrun simctl status_bar booted clear
