#!/bin/bash
# App Store frames: a caption above each simulator screenshot, 1320 x 2868.
#
#   scripts/frame-screenshots.sh en
#
# Reads screenshots/<version>/<lang>/NN.png and one caption per line from
# screenshots/<version>/captions-<lang>.txt, renders each frame with headless
# Chrome and writes screenshots/<version>/<lang>/store/NN.png. A caption that
# starts with "@ " gets the app icon and name above it. SHOTS_DIR, CAPTIONS
# and OUT_DIR point elsewhere, for drafts that must leave the store untouched.
set -euo pipefail

LANG_CODE="${1:?usage: $0 en|fr}"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DIR="${SHOTS_DIR:-screenshots/$VERSION/$LANG_CODE}"
CAPTIONS="${CAPTIONS:-screenshots/$VERSION/captions-$LANG_CODE.txt}"
OUT="${OUT_DIR:-$DIR/store}"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
ICON="$PWD/assets/flowflow-icon-300.png"
W=1320
H=2868
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$OUT"
i=0
while IFS= read -r caption; do
  i=$((i + 1))
  n=$(printf '%02d' "$i")
  shot="$(cd "$DIR" && pwd)/$n.png"
  [ -f "$shot" ] || { echo "WARN no $shot"; continue; }
  brand=""
  if [[ "$caption" == "@ "* ]]; then
    caption="${caption#@ }"
    brand="<header><img src=\"file://$ICON\"><span>FlowFlow</span></header>"
  fi
  # The app's own tokens (tailwind.css): warm white, stone ink, the card
  # radius and lift shadow, the system font. With the brand row the caption
  # and the phone move up and shrink to keep the same bottom margin.
  cat > "$WORK/$n.html" <<HTML
<!doctype html><meta charset="utf-8">
<style>
  html, body { margin: 0; width: ${W}px; height: ${H}px; overflow: hidden; }
  body { background: oklch(0.99 0.005 50); font-family: -apple-system, "SF Pro Display", sans-serif;
         display: flex; flex-direction: column; align-items: center; }
  h1 { margin: 190px 110px 0; height: 300px; display: flex; align-items: center;
       text-align: center; color: #1c1917; font-size: 104px; line-height: 1.12;
       font-weight: 700; letter-spacing: -0.02em; }
  .phone { margin-top: 70px; height: 2200px; border-radius: 96px; border: 2px solid #e7e5e4;
        box-shadow: 0 1px 2px rgba(28,25,23,.06), 0 30px 80px -30px rgba(28,25,23,.28); }
  header { margin-top: 120px; display: flex; align-items: center; gap: 28px; }
  header img { width: 132px; height: 132px; border-radius: 30px; }
  header span { color: #1c1917; font-size: 72px; font-weight: 700; letter-spacing: -0.02em; }
  header ~ h1 { margin-top: 30px; height: 260px; }
  header ~ .phone { margin-top: 40px; height: 2040px; border-radius: 90px; }
</style>
$brand
<h1>$caption</h1>
<img class="phone" src="file://$shot">
HTML
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --allow-file-access-from-files \
    --force-device-scale-factor=1 --window-size=$W,$H \
    --screenshot="$WORK/$n.png" "file://$WORK/$n.html" >/dev/null 2>&1
  # Through JPEG and back: App Store Connect wants no alpha channel.
  sips -s format jpeg -s formatOptions best "$WORK/$n.png" --out "$WORK/$n.jpg" >/dev/null
  sips -s format png "$WORK/$n.jpg" --out "$OUT/$n.png" >/dev/null
  echo "$OUT/$n.png  $caption"
done < "$CAPTIONS"
