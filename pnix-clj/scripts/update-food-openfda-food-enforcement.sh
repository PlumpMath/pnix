#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIMIT="${OPENFDA_FOOD_ENFORCEMENT_LIMIT:-500}"
OUT="$ROOT/ingest/food/openfda-food-enforcement"
mkdir -p "$OUT"
receipt="$ROOT/corpus/food/LICENSES/openfda-food-enforcement.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "openfda-food-enforcement" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/food-enforcement.json" "https://api.fda.gov/food/enforcement.json?limit=${LIMIT}"
( cd "$OUT" && shasum -a 256 food-enforcement.json > SHA256SUMS )
printf 'openfda-food-enforcement updated: %s\n' "$OUT"
