#!/usr/bin/env bash
# Copy pnixc-pnix Stage-2 eval runtime from ~/pnix into pnix-clj (read-only on ~/pnix).
# Enables pnixc-meta one-shot .px eval with PNIX_WORKSPACE_ROOT=pnix-clj.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PNIX="${PNIX_ROOT:-$HOME/pnix}"
SRC="$PNIX/pnixc-pnix"
DEST="$ROOT/pnixc-pnix"

if [[ ! -d "$SRC/exec" ]]; then
  echo "missing $SRC (set PNIX_ROOT)" >&2
  exit 1
fi

rm -rf "$DEST"
mkdir -p "$DEST"
rsync -a --exclude '.DS_Store' "$SRC/" "$DEST/"
echo "synced pnixc-pnix -> $DEST ($(find "$DEST" -type f | wc -l | tr -d ' ') files)"
