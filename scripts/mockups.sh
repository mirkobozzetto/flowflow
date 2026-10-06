#!/usr/bin/env bash
# Rebuilds the mockup kit from the app: icons from src/ui/icons.rs, styles
# from tailwind.css with docs/mockups/*.html as extra sources, so any class
# a mockup uses exists exactly as the app would render it.
set -euo pipefail

cd "$(dirname "$0")/.."
TAILWIND="${TAILWIND:-$(ls -d "$HOME"/.dx/tools/tailwindcss-v*/tailwindcss 2>/dev/null | tail -n 1)}"
[ -x "$TAILWIND" ] || { echo "tailwindcss not found: build the app once with dx, or set TAILWIND." >&2; exit 1; }

python3 scripts/mockup_icons.py
"$TAILWIND" -i docs/mockups/kit/mockups.css -o docs/mockups/kit/app.css
