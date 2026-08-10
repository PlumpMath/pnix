#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/ingest/device/accessgudid-api-catalog"
mkdir -p "$OUT"
receipt="$ROOT/corpus/device/LICENSES/accessgudid-api-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "accessgudid-api-catalog" "$receipt"
for page in device_lookup_api device_history_api parse_udi_api; do
  curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/${page}.html" "https://accessgudid.nlm.nih.gov/resources/developers/${page}"
done
( cd "$OUT" && shasum -a 256 *.html > SHA256SUMS )
printf 'accessgudid-api-catalog updated: %s\n' "$OUT"
