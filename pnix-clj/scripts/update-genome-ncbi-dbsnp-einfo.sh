#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${NCBI_DBSNP_EINFO_OUT:-$ROOT/ingest/genome/ncbi-dbsnp-einfo}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/genome/LICENSES/ncbi-dbsnp-einfo.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "ncbi-dbsnp-einfo" "$receipt"
url="${NCBI_DBSNP_EINFO_URL:-https://eutils.ncbi.nlm.nih.gov/entrez/eutils/einfo.fcgi?db=snp&retmode=json}"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/snp.einfo.json" "$url"
( cd "$OUT" && shasum -a 256 snp.einfo.json > SHA256SUMS )
printf 'ncbi-dbsnp-einfo updated: %s\n' "$OUT"
