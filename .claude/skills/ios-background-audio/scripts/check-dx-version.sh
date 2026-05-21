#!/usr/bin/env bash
set -euo pipefail

EXPECTED="0.7.9"

if ! command -v dx >/dev/null 2>&1; then
    echo "dx not found on PATH"
    echo "Install: cargo install dioxus-cli@${EXPECTED} --force"
    exit 2
fi

RAW="$(dx --version 2>&1 | head -n 1)"
ACTUAL="$(printf '%s\n' "$RAW" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1 || true)"

if [[ -z "$ACTUAL" ]]; then
    echo "Could not parse dx version from: $RAW"
    exit 2
fi

echo "dx version: $ACTUAL (expected: $EXPECTED)"

if [[ "$ACTUAL" == "$EXPECTED" ]]; then
    echo "OK"
    exit 0
fi

echo "MISMATCH"
echo "Upgrade: dx self-update    OR    cargo install dioxus-cli@${EXPECTED} --force"
exit 1
