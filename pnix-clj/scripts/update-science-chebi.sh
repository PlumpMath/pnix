#!/usr/bin/env bash
set -euo pipefail
# Download the current ChEBI core OBO snapshot into the local ingest area.
# Data is intentionally gitignored; scripts + provenance are committed.
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/science/chebi"
URL="${CHEBI_URL:-https://purl.obolibrary.org/obo/chebi/chebi_core.obo}"
OUT="$OUT_DIR/chebi_core.obo"
TMP="$OUT.tmp.$$"
mkdir -p "$OUT_DIR"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$OUT"
shasum -a 256 "$OUT" > "$OUT.sha256"
{
  echo "source_url=$URL"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$OUT.sha256")"
} > "$OUT.meta"
echo "wrote: $OUT"
cat "$OUT.sha256"
