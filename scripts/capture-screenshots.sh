#!/bin/bash
# App Store screenshots and video clips from the iOS simulator, one locale
# per run.
#
#   scripts/capture-screenshots.sh en /tmp/flowflow-demo-en.db [clips-dir]
#   ONLY="first-note sources" scripts/capture-screenshots.sh ...   (retakes)
#
# Needs a debug simulator build (docs/release/) and a demo store from
# `cargo run --example demo_store`; this script serves its fake Hermes, so no
# personal note, Hermes address or skill reaches a screenshot.
# Each screen is set up by the debug-only screenshot watcher
# (src/ui/app/watchers.rs), which reads a `shot` file next to the store.
# No hands: the native "+" menu cannot open headless, so the skills show
# through "/" in Hermes' field. Output: screenshots/<version>/<lang>/NN.png,
# and with a clips-dir one raw clip per scene in clips-dir/<lang>/, edited
# into the previews and the promo by the video project.
set -euo pipefail

LANG_CODE="${1:?usage: $0 en|fr demo.db [clips-dir]}"
DEMO_DB="${2:?usage: $0 en|fr demo.db [clips-dir]}"
CLIPS="${3:-}"
DEVICE="iPhone 17 Pro Max"
BUNDLE=com.mirkobozzetto.flowflow
APP=target/dx/flowflow/debug/ios/Flowflow.app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="screenshots/$VERSION/$LANG_CODE"
EXPECTED="1320x2868"
FRESH_DB="${TMPDIR:-/tmp}/flowflow-fresh-$LANG_CODE.db"

case "$LANG_CODE" in
  fr)
    PHOTO=tableau.jpg
    ASK="Prépare la facture Nordwind"
    NEW="Prépare mon appel Nordwind"
    QUESTION="Fais-en mon plan de lancement"
    VOICE=Thomas
    DICTATION="Rappelle-moi d'envoyer la proposition jeudi à neuf heures."
    START="C'est parti"
    SOURCE="Retours des bêta-testeurs"
    THEME="Lancement produit"
    SKILL=planning-semaine
    ;;
  *)
    PHOTO=whiteboard.jpg
    ASK="Draft the Nordwind invoice"
    NEW="Prep my Nordwind call"
    QUESTION="Turn this into my launch plan"
    VOICE=Samantha
    DICTATION="Remind me to send the proposal Thursday at nine."
    START="Let's go"
    SOURCE="Feedback from the beta testers"
    THEME="Product launch"
    SKILL=weekly-planner
    ;;
esac

# App Store order (docs/release/<version>.md): the core of the app first,
# Hermes after. Lines after a screen are the watcher's arguments.
SHOTS=(
  record
  note
  chat
  menu
  "hermes
photo=$PHOTO"
  "hermes
text=/"
  "hermes
text=$ASK"
  "hermes-new
text=$NEW"
)

cargo build -q --example demo_store
target/debug/examples/demo_store hermes "$LANG_CODE" >/dev/null &
HERMES_PID=$!
trap 'kill $HERMES_PID 2>/dev/null || true' EXIT

# Fresh store on every start: a take left running must not leak into the
# next screens.
reset() {
  xcrun simctl terminate booted "$BUNDLE" 2>/dev/null || true
  rm -f "$DOCS"/flowflow.db* "$DOCS/shot"
  cp "${1:-$DEMO_DB}" "$DOCS/flowflow.db"
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

# A retake (ONLY set) films the named clips and leaves the screenshots.
want() { [ -z "${ONLY:-}" ] || [[ " $ONLY " == *" $1 "* ]]; }

mkdir -p "$OUT"
want screenshots && rm -f "$OUT"/*.png
for i in "${!SHOTS[@]}"; do
  want screenshots || break
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

# A clip plays shots on a timeline: "seconds|shot", one line per step; a
# "!" shot runs a command instead (the Mac speaks into the simulator's mic).
# The simulator writes frames only when the screen moves: the editor holds
# the last one. CLIP_DB picks the store the clip starts from.
clip() {
  local name=$1 rec
  shift
  want "$name" || return 0
  reset "${CLIP_DB:-}"
  xcrun simctl io booted recordVideo --codec=h264 --force "$CLIPS/$name.mov" 2>/dev/null &
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
  echo "$CLIPS/$name.mov"
}

if [ -n "$CLIPS" ]; then
  CLIPS="$CLIPS/$LANG_CODE"
  mkdir -p "$CLIPS"
  target/debug/examples/demo_store "$FRESH_DB" "$LANG_CODE" fresh >/dev/null
  CLIP_DB="$FRESH_DB" clip first-note "3|tap
label=$START" "2|record" "7|!say -v $VOICE \"$DICTATION\"" "14|tap
selector=.voice-capsule button:last-of-type"
  clip sources "3|chat" "2|tap
label=sources" "4|tap
label=$SOURCE"
  clip themes "2|home" "2|menu" "4|tap
label=$THEME"
  clip account "4|settings
section=account"
  clip ai "4|settings
section=ai"
  clip transcription "4|settings
section=transcription"
  clip connections "5|settings
section=connections"
  clip hermes "4|hermes-new
photo=$PHOTO
text=$QUESTION" "13|tap
selector=.composer-act-send"
  clip skills "3|hermes
text=/" "3|tap
label=$SKILL"
fi

xcrun simctl status_bar booted clear
