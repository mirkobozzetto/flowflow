#!/usr/bin/env bash
# Rebuilds public/videos and public/shots from the release media.
# Sources: docs/release/media.md. Raw clips live in the Remotion project.
set -euo pipefail

SITE="$(cd "$(dirname "$0")/.." && pwd)"
REPO="$(cd "$SITE/.." && pwd)"
RELEASE="$REPO/screenshots/2.1.1"
CLIPS="${CLIPS:-$HOME/code/flowflow-videos/public/clips}"
WIDTH=720
ENCODE=(-c:v libx264 -profile:v high -level 4.0 -pix_fmt yuv420p
  -preset slow -crf 23 -movflags +faststart)

# cut_clip <src> <out> <from:to:speed>... -> trimmed, retimed, silent mp4
cut_clip() {
  local src="$1" out="$2"
  shift 2
  local filter="" labels="" n=0 piece from to speed
  for piece in "$@"; do
    IFS=: read -r from to speed <<<"$piece"
    filter+="[0:v]trim=$from:$to,setpts=(PTS-STARTPTS)/$speed,"
    filter+="scale=$WIDTH:-2,fps=30[p$n];"
    labels+="[p$n]"
    n=$((n + 1))
  done
  filter+="${labels}concat=n=$n:v=1:a=0[v]"
  ffmpeg -v error -y -i "$src" -filter_complex "$filter" -map "[v]" \
    "${ENCODE[@]}" -an "$out"
}

# ffmpeg builds without libwebp are common: cwebp does the webp step.
poster() {
  local png="${1%.mp4}.png"
  ffmpeg -v error -y -ss "$2" -i "$1" -frames:v 1 -vf "scale=$WIDTH:-2" "$png"
  cwebp -quiet -q 84 "$png" -o "${1%.mp4}.webp"
  rm "$png"
}

for lang in en fr; do
  out="$SITE/public/videos/$lang"
  shots="$SITE/public/shots/$lang"
  mkdir -p "$out" "$shots"

  ffmpeg -v error -y -i "$RELEASE/promo/flowflow-promo-$lang.mp4" \
    "${ENCODE[@]}" -crf 25 \
    -c:a aac -b:a 96k "$out/promo.mp4"
  poster "$out/promo.mp4" 4

  if [ "$lang" = fr ]; then
    dictate=(0:1:1 2:4.4:2 4.6:13:4 13:17.5:2.5)
  else
    dictate=(0:1:1 2:4.4:2 4.6:14:4 14:19.5:3)
  fi
  cut_clip "$CLIPS/$lang/first-note.mp4" "$out/dictate.mp4" "${dictate[@]}"
  poster "$out/dictate.mp4" 5.2

  cut_clip "$CLIPS/$lang/sources.mp4" "$out/answer.mp4" \
    1:4.4:1.4 4.4:9:1
  poster "$out/answer.mp4" 3.8

  cut_clip "$CLIPS/$lang/hermes.mp4" "$out/agent.mp4" \
    1:4.2:1.3 4.2:12:1.6
  poster "$out/agent.mp4" 6.4

  # No raw meeting clip exists: two real captures, cross-faded.
  ffmpeg -v error -y \
    -loop 1 -t 3.6 -i "$RELEASE/$lang/03.png" \
    -loop 1 -t 3.6 -i "$RELEASE/$lang/04.png" \
    -loop 1 -t 1.2 -i "$RELEASE/$lang/03.png" \
    -filter_complex "[0:v]scale=$WIDTH:-2,fps=30,format=yuv420p[a];
      [1:v]scale=$WIDTH:-2,fps=30,format=yuv420p[b];
      [2:v]scale=$WIDTH:-2,fps=30,format=yuv420p[c];
      [a][b]xfade=transition=fade:duration=0.6:offset=3[ab];
      [ab][c]xfade=transition=fade:duration=0.6:offset=6[v]" \
    -map "[v]" "${ENCODE[@]}" -an "$out/meeting.mp4"
  poster "$out/meeting.mp4" 1

  for n in 02 05 06 07 09; do
    cwebp -quiet -q 86 -resize 900 0 "$RELEASE/$lang/$n.png" \
      -o "$shots/$n.webp"
  done
done

du -ch "$SITE"/public/videos/*/* | tail -1
