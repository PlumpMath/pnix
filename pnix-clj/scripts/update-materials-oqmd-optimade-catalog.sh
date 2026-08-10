#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OQMD_OPTIMADE_OUT:-$ROOT/ingest/materials/oqmd-optimade-catalog}"
BASE="${OQMD_OPTIMADE_BASE:-https://oqmd.org/optimade/v1}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/materials/LICENSES/oqmd-optimade-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "oqmd-optimade-catalog" "$receipt"
for ep in info links; do
  curl -L --fail --retry 3 --retry-delay 2 --max-time 30 -o "$OUT/${ep}.json" "$BASE/${ep}"
done
( cd "$OUT" && shasum -a 256 info.json links.json > SHA256SUMS )
printf 'oqmd-optimade-catalog updated: %s\n' "$OUT"
