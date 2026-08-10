#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/ingest/bio/insdc-feature-table"
mkdir -p "$OUT"
receipt="$ROOT/corpus/bio/LICENSES/insdc-feature-table.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "insdc-feature-table" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/feature-table.html" "https://www.insdc.org/submitting-standards/feature-table/"
( cd "$OUT" && shasum -a 256 feature-table.html > SHA256SUMS )
printf 'insdc-feature-table updated: %s\n' "$OUT"
