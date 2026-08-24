#!/usr/bin/env bash
# Second FlowFlow instance on this Mac, with its own store and therefore its
# own device identity and its own account.
#
# FLOWFLOW_DATA_DIR is the seam the persistence layer already exposes for
# exactly this. The real store in ~/Library/Application Support/FlowFlow is
# never touched, so testing a space between two accounts costs no note.
#
# The .app bundle would not inherit this variable through Finder, so the
# binary is launched directly.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/dx/flowflow/release/macos/Flowflow.app/Contents/MacOS/flowflow"
DATA_DIR="${FLOWFLOW_TESTER_DIR:-$HOME/FlowFlowTester}"

if [ ! -x "$BIN" ]; then
  echo "Build it first: make desktop-app" >&2
  exit 1
fi

mkdir -p "$DATA_DIR"
echo "tester store: $DATA_DIR"
exec env FLOWFLOW_DATA_DIR="$DATA_DIR" "$BIN"
