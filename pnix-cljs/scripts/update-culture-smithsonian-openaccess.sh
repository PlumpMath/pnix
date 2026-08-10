#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ROWS="${SMITHSONIAN_ROWS:-100}"
OUT="$ROOT/ingest/culture/smithsonian-openaccess"
mkdir -p "$OUT"
receipt="$ROOT/corpus/culture/LICENSES/smithsonian-openaccess.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "smithsonian-openaccess" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/search.json" "https://api.si.edu/openaccess/api/v1.0/search?api_key=DEMO_KEY&q=*&rows=${ROWS}"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/apidocs.html" "https://edan.si.edu/openaccess/apidocs/"
( cd "$OUT" && shasum -a 256 search.json apidocs.html > SHA256SUMS )
printf 'smithsonian-openaccess updated: %s\n' "$OUT"
