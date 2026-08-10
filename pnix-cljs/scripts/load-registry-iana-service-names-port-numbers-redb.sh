#!/usr/bin/env bash
# Generate each chunk and append it to redb. Uses pnix attrset source; no graph/mirror/math wiring.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
CHUNK_SIZE="${CHUNK_SIZE:-1500}"
XML="$ROOT/ingest/registry/iana-service-names-port-numbers/service-names-port-numbers.xml"
PLAN="$ROOT/stdlib/lib/corpus/iana-service-names-port-numbers-store-plan.px"
TOTAL=$(python3 - "$XML" <<'PY'
import sys, xml.etree.ElementTree as ET
ns='{http://www.iana.org/assignments}'
print(len(ET.parse(sys.argv[1]).getroot().findall(ns+'record')))
PY
)
COUNT=$(( (TOTAL + CHUNK_SIZE - 1) / CHUNK_SIZE ))
echo "IANA service-names-port-numbers: total=$TOTAL chunk_size=$CHUNK_SIZE chunks=$COUNT"
for ((i=0; i<COUNT; i++)); do
  "$ROOT/scripts/gen-registry-iana-service-names-port-numbers.sh" --chunk-size "$CHUNK_SIZE" --chunk-index "$i"
  "$ROOT/target/debug/pnixc-meta" --morph-rules-build "$PLAN"
done
"$ROOT/target/debug/pnixc-meta" --morph-rules-verify "$PLAN"
echo "loaded chunks=$COUNT total=$TOTAL"
