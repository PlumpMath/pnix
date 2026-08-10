#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/ingest/consumer/cpsc-recalls-api"
mkdir -p "$OUT"
receipt="$ROOT/corpus/consumer/LICENSES/cpsc-recalls-api.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "cpsc-recalls-api" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/recalls.json" "https://www.saferproducts.gov/RestWebServices/Recall?format=json"
( cd "$OUT" && shasum -a 256 recalls.json > SHA256SUMS )
printf 'cpsc-recalls-api updated: %s\n' "$OUT"
