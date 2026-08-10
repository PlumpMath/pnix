#!/usr/bin/env bash
# Generate NASA Exoplanet Archive chunks and append each to redb. No graph/mirror/math wiring.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
CHUNK_SIZE="${CHUNK_SIZE:-1000}"
CSV="$ROOT/ingest/science/nasa-exoplanet-archive/pscomppars-selected.csv"
PLAN="$ROOT/stdlib/lib/corpus/nasa-exoplanet-archive-store-plan.px"
TOTAL=$(python3 - "$CSV" <<'PY'
import csv,sys
with open(sys.argv[1],encoding='utf-8') as f:
    print(sum(1 for _ in csv.DictReader(f)))
PY
)
COUNT=$(( (TOTAL + CHUNK_SIZE - 1) / CHUNK_SIZE ))
echo "NASA Exoplanet Archive pscomppars: total=$TOTAL chunk_size=$CHUNK_SIZE chunks=$COUNT"
for ((i=0; i<COUNT; i++)); do
  "$ROOT/scripts/gen-science-nasa-exoplanet-archive.sh" --chunk-size "$CHUNK_SIZE" --chunk-index "$i"
  "$ROOT/target/debug/pnixc-meta" --morph-rules-build "$PLAN"
done
"$ROOT/target/debug/pnixc-meta" --morph-rules-verify "$PLAN"
echo "loaded chunks=$COUNT total=$TOTAL"
