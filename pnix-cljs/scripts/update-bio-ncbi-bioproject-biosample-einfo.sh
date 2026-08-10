#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/ingest/bio/ncbi-bioproject-biosample-einfo"
mkdir -p "$OUT"
receipt="$ROOT/corpus/bio/LICENSES/ncbi-bioproject-biosample-einfo.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "ncbi-bioproject-biosample-einfo" "$receipt"
for db in bioproject biosample; do
  curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/${db}.einfo.json" "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/einfo.fcgi?db=${db}&retmode=json"
done
( cd "$OUT" && shasum -a 256 *.json > SHA256SUMS )
printf 'ncbi-bioproject-biosample-einfo updated: %s\n' "$OUT"
