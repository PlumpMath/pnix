#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${CELLOSAURUS_OUT:-$ROOT/ingest/bio/cellosaurus}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/bio/LICENSES/cellosaurus-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "cellosaurus-catalog" "$receipt"
url="${CELLOSAURUS_URL:-https://ftp.expasy.org/databases/cellosaurus/cellosaurus.txt}"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/cellosaurus.txt" "$url"
( cd "$OUT" && shasum -a 256 cellosaurus.txt > SHA256SUMS )
printf 'cellosaurus-catalog updated: %s\n' "$OUT"
