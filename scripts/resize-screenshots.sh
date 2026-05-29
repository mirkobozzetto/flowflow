#!/usr/bin/env bash
set -euo pipefail

DIR=${1:-screenshots}
TARGET_W=1284
TARGET_H=2778
PAD_COLOR=FFFFFF
SUFFIX="-ok"

if [ ! -d "$DIR" ]; then
  echo "ERROR: dir '$DIR' does not exist."
  exit 1
fi

shopt -s nullglob nocaseglob
files=("$DIR"/*.{png,jpg,jpeg})
shopt -u nullglob nocaseglob

filtered=()
for f in "${files[@]}"; do
  base=$(basename "$f")
  case "$base" in
    *"${SUFFIX}".*) continue ;;
  esac
  filtered+=("$f")
done

if [ ${#filtered[@]} -eq 0 ]; then
  echo "ERROR: no source png/jpg files (without ${SUFFIX} suffix) in '$DIR'."
  exit 1
fi

echo ">> Target: ${TARGET_W} x ${TARGET_H} PNG sRGB (iPhone 6.5\" slot)"
echo ">> Dir: $DIR, suffix: ${SUFFIX}"
echo

for f in "${filtered[@]}"; do
  name=$(basename "$f")
  base="${name%.*}"
  out_file="$DIR/${base}${SUFFIX}.png"
  tmp_file="$DIR/.tmp-${base}.png"

  sips --resampleWidth "$TARGET_W" "$f" --out "$tmp_file" >/dev/null
  sips --padToHeightWidth "$TARGET_H" "$TARGET_W" --padColor "$PAD_COLOR" "$tmp_file" --out "$tmp_file" >/dev/null
  sips -s format png -m /System/Library/ColorSync/Profiles/sRGB\ Profile.icc "$tmp_file" --out "$out_file" >/dev/null
  rm -f "$tmp_file"

  meta=$(sips -g pixelWidth -g pixelHeight -g format -g profile "$out_file" 2>/dev/null | awk '/pixel|format:|profile:/ {print $0}' | paste -sd " | " -)
  echo "  $name -> ${base}${SUFFIX}.png"
  echo "    $meta"
done

echo
echo ">> Done. Files with ${SUFFIX} suffix are validated for ASC. Upload them only."
