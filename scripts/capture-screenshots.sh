#!/bin/bash
set -e

DEVICE="iPhone 17 Pro Max"
OUT="$HOME/Desktop"
EXPECTED_W=1320
EXPECTED_H=2868

SLOTS=(
  "Recording bar + waveform"
  "NoteDetail transcribed + tags"
  "ChatView + sources"
  "NotesList + tag chips"
  "Sidebar folders open"
  "NoteDetail + AttachmentModal PDF"
  "Settings provider picker"
)

if ! xcrun simctl list devices | grep "$DEVICE" | grep -q Booted; then
  echo ">> Booting $DEVICE..."
  xcrun simctl boot "$DEVICE" 2>/dev/null || true
  open -a Simulator
  sleep 3
fi

for i in 1 2 3 4 5 6 7; do
  IDX=$((i-1))
  echo ""
  echo "=========================================="
  echo "  Slot $i / 7 — ${SLOTS[$IDX]}"
  echo "=========================================="
  echo "  Navigate the simulator to this view, then press Enter."
  read -r _
  FILE="$OUT/slot-$i.png"
  xcrun simctl io booted screenshot "$FILE"
  W=$(sips -g pixelWidth "$FILE" 2>/dev/null | awk '/pixelWidth/{print $2}')
  H=$(sips -g pixelHeight "$FILE" 2>/dev/null | awk '/pixelHeight/{print $2}')
  if [ "$W" = "$EXPECTED_W" ] && [ "$H" = "$EXPECTED_H" ]; then
    echo "  OK $FILE ($W x $H)"
  else
    echo "  WARN $FILE size $W x $H (expected $EXPECTED_W x $EXPECTED_H)"
    echo "  ASC will refuse this resolution. Boot a real iPhone 6.9 sim and retry."
  fi
done

echo ""
echo "=========================================="
echo "  Done. 7 screenshots saved to $OUT/"
echo "  Drag-drop them into ASC Version 1.0 -> iPhone 6.9 in order."
echo "=========================================="
