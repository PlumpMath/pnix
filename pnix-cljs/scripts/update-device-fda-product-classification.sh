#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIMIT="${FDA_DEVICE_CLASSIFICATION_LIMIT:-500}"
OUT="$ROOT/ingest/device/fda-product-classification"
mkdir -p "$OUT"
receipt="$ROOT/corpus/device/LICENSES/fda-product-classification.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "fda-product-classification" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/device-classification.json" "https://api.fda.gov/device/classification.json?limit=${LIMIT}"
( cd "$OUT" && shasum -a 256 device-classification.json > SHA256SUMS )
printf 'fda-product-classification updated: %s\n' "$OUT"
