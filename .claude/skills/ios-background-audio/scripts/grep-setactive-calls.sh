#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
SRC="$ROOT/src"

if [[ ! -d "$SRC" ]]; then
    echo "src directory not found at: $SRC"
    exit 2
fi

echo "Scanning $SRC for setActive(...) call sites"
echo "----"

MATCHES="$(grep -rn -E 'setActive\s*\(' "$SRC" --include='*.rs' --include='*.swift' --include='*.m' --include='*.mm' || true)"

if [[ -z "$MATCHES" ]]; then
    echo "No setActive calls found."
    exit 0
fi

printf '%s\n' "$MATCHES"
echo "----"

FALSE_MATCHES="$(printf '%s\n' "$MATCHES" | grep -E 'setActive\s*\(\s*(false|NO|0)' || true)"

if [[ -n "$FALSE_MATCHES" ]]; then
    echo
    echo "WARNING: setActive(false|NO|0) call sites detected. Audit each one to confirm no recording is active."
    printf '%s\n' "$FALSE_MATCHES"
    exit 1
fi

echo "No setActive(false) calls. Safe."
exit 0
