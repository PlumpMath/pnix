#!/usr/bin/env bash
# Generate and append every eCFR 49 CFR 172.101 hazmat identifier chunk. No graph/mirror/math wiring.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
CHUNK_SIZE="${CHUNK_SIZE:-500}"
PLAN="$ROOT/stdlib/lib/corpus/ecfr-49-172-101-hazmat-identifiers-store-plan.px"
TOTAL=$(
  "$ROOT/scripts/gen-regulatory-ecfr-hazmat-identifiers.sh" --chunk-size "$CHUNK_SIZE" --chunk-index 0 >/tmp/ecfr-hazmat-gen0.log
  python3 - "$ROOT/stdlib/lib/corpus/ecfr-49-172-101-hazmat-identifiers.generated.px" <<'PY'
import re,sys
s=open(sys.argv[1],encoding='utf-8').read()
m=re.search(r'"identifier_row_count_total" = ([0-9]+);',s)
print(m.group(1) if m else '0')
PY
)
COUNT=$(( (TOTAL + CHUNK_SIZE - 1) / CHUNK_SIZE ))
echo "eCFR hazmat identifiers: total=$TOTAL chunk_size=$CHUNK_SIZE chunks=$COUNT"
for ((i=0; i<COUNT; i++)); do
  "$ROOT/scripts/gen-regulatory-ecfr-hazmat-identifiers.sh" --chunk-size "$CHUNK_SIZE" --chunk-index "$i"
  "$ROOT/target/debug/pnixc-meta" --morph-rules-build "$PLAN"
done
"$ROOT/target/debug/pnixc-meta" --morph-rules-verify "$PLAN"
echo "loaded chunks=$COUNT total=$TOTAL"
