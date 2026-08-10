#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${PNIXC_META:-$ROOT/target/debug/pnixc-meta}"
PLAN="$ROOT/stdlib/lib/corpus/unicode-ucd-store-plan.px"
CHUNK_DIR="$ROOT/stdlib/lib/corpus/unicode-ucd.generated"
ACTIVE="$ROOT/stdlib/lib/corpus/unicode-ucd-current-chunk.generated.px"
export PNIX_EVAL_MAX_DEPTH="${PNIX_EVAL_MAX_DEPTH:-300000}"
if [[ ! -d "$CHUNK_DIR" ]]; then
  "$ROOT/scripts/gen-text-unicode-ucd.sh"
fi
count=0
for chunk in "$CHUNK_DIR"/*.px; do
  cp "$chunk" "$ACTIVE"
  "$BIN" --morph-rules-build "$PLAN"
  "$BIN" --morph-rules-verify "$PLAN"
  count=$((count+1))
done
rm -f "$ACTIVE"
echo "unicode-ucd redb load complete: chunks=$count"
