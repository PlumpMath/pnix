#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${MP_API_CATALOG_OUT:-$ROOT/ingest/materials/materials-project-api-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/materials/LICENSES/materials-project-api-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "materials-project-api-catalog" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 --max-time 60 -o "$OUT/openapi.json" "${MP_OPENAPI_URL:-https://api.materialsproject.org/openapi.json}"
( cd "$OUT" && shasum -a 256 openapi.json > SHA256SUMS )
printf 'materials-project-api-catalog updated: %s\n' "$OUT"
