#!/bin/bash
# Signs a disk image with the Developer ID Application identity. Gatekeeper
# checks the DMG's own signature (spctl -t open), not only the app inside, and
# notarization must run after this signature. Without that identity the image
# stays unsigned, as `make dmg` then falls back to Apple Development anyway.
set -euo pipefail

DMG="${1:?usage: $0 <file.dmg>}"
IDENTITY=$(security find-identity -v -p codesigning |
  awk -F'"' '/Developer ID Application/{print $2; exit}')
if [ -z "$IDENTITY" ]; then
  echo ">> No Developer ID Application certificate: $DMG left unsigned."
  exit 0
fi
codesign --force --timestamp --sign "$IDENTITY" "$DMG"
echo ">> $DMG signed by $IDENTITY"
