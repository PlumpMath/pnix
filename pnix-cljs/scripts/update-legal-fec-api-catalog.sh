#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${FEC_API_CATALOG_OUT:-$ROOT/ingest/legal/fec-api-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/legal/LICENSES/fec-api-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "fec-api-catalog" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 --max-time 60 -o "$OUT/swagger.json" "${FEC_SWAGGER_URL:-https://api.open.fec.gov/swagger/}"
curl -L --fail --retry 3 --retry-delay 2 --max-time 30 -o "$OUT/developers.html" "${FEC_DEVELOPERS_URL:-https://api.open.fec.gov/developers/}"
( cd "$OUT" && shasum -a 256 swagger.json developers.html > SHA256SUMS )
printf 'fec-api-catalog updated: %s\n' "$OUT"
