#!/bin/bash
# App Store frames: the app icon and name, a caption, then the simulator
# screenshot, 1320 x 2868.
#
#   scripts/frame-screenshots.sh en
#
# Reads screenshots/<version>/<lang>/NN.png and one caption per line from
# screenshots/<version>/captions-<lang>.txt, renders each frame with headless
# Chrome and writes screenshots/<version>/<lang>/store/NN.png. A *word* in a
# caption shows in the icon's orange. SHOTS_DIR, CAPTIONS and OUT_DIR point
# elsewhere, for drafts that must leave the store untouched; LAYOUT=left
# aligns the text left and BLEED=1 lets the phone run off the bottom edge.
set -euo pipefail

LANG_CODE="${1:?usage: $0 en|fr}"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DIR="${SHOTS_DIR:-screenshots/$VERSION/$LANG_CODE}"
CAPTIONS="${CAPTIONS:-screenshots/$VERSION/captions-$LANG_CODE.txt}"
OUT="${OUT_DIR:-$DIR/store}"
LAYOUT="${LAYOUT:-center}"
BLEED="${BLEED:-0}"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
ICON="$PWD/assets/flowflow-icon-300.png"
W=1320
H=2868
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# Paper grain: fractal noise, a few percent over the warm white.
GRAIN="url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='300' height='300'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 .11 0 0 0 0 .10 0 0 0 0 .09 0 0 0 .13 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E\")"
ALIGN=center
PAD=110
[ "$LAYOUT" = left ] && ALIGN=flex-start
PHONE_H=1960
PHONE_RADIUS="88px"
if [ "$BLEED" = 1 ]; then
  PHONE_H=2300
  PHONE_RADIUS="96px 96px 0 0"
fi

mkdir -p "$OUT"
i=0
while IFS= read -r caption; do
  i=$((i + 1))
  n=$(printf '%02d' "$i")
  shot="$(cd "$DIR" && pwd)/$n.png"
  [ -f "$shot" ] || { echo "WARN no $shot"; continue; }
  title=$(printf '%s' "$caption" | sed -E 's/\*([^*]+)\*/<em>\1<\/em>/g')
  # The app's own tokens (tailwind.css): warm white, stone ink, ios-orange,
  # the system font set heavy and tight.
  cat > "$WORK/$n.html" <<HTML
<!doctype html><meta charset="utf-8">
<style>
  html, body { margin: 0; width: ${W}px; height: ${H}px; overflow: hidden; }
  body { background: ${GRAIN}, oklch(0.99 0.005 50);
         font-family: -apple-system, "SF Pro Display", sans-serif; color: #1c1917;
         display: flex; flex-direction: column; align-items: ${ALIGN};
         padding: 0 ${PAD}px; box-sizing: border-box;
         -webkit-font-smoothing: antialiased; }
  header { margin-top: 150px; display: flex; align-items: center; gap: 30px; }
  header img { width: 150px; height: 150px; border-radius: 34px;
               box-shadow: 0 1px 2px rgba(28,25,23,.08), 0 14px 30px -14px rgba(28,25,23,.35); }
  header span { font-size: 128px; font-weight: 900; letter-spacing: -0.05em; line-height: 1; }
  h1 { margin: 40px 0 0; min-height: 184px; font-size: 84px; font-weight: 800;
       letter-spacing: -0.035em; line-height: 1.07; text-align: ${LAYOUT};
       text-wrap: balance; max-width: 1100px; color: #292524; }
  h1 em { font-style: normal; color: oklch(0.60 0.22 50); white-space: nowrap; }
  .phone { margin-top: 64px; align-self: center; height: ${PHONE_H}px;
           border-radius: ${PHONE_RADIUS}; border: 2px solid #e7e5e4;
           box-shadow: 0 1px 2px rgba(28,25,23,.06), 0 24px 48px -24px rgba(28,25,23,.22),
                       0 60px 120px -50px rgba(28,25,23,.30); }
</style>
<header><img src="file://$ICON"><span>FlowFlow</span></header>
<h1>$title</h1>
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
