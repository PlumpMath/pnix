#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/science/ncbi-taxonomy"
URL="${NCBI_TAXONOMY_URL:-https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/taxdump.tar.gz}"
TAR="$OUT_DIR/taxdump.tar.gz"
TMP="$TAR.tmp.$$"
mkdir -p "$OUT_DIR/taxdump"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$TAR"
shasum -a 256 "$TAR" > "$TAR.sha256"
rm -f "$OUT_DIR/taxdump"/*.dmp
# Keep source schema files only; no sequence data.
tar -xzf "$TAR" -C "$OUT_DIR/taxdump" nodes.dmp names.dmp merged.dmp delnodes.dmp
{
  echo "source_url=$URL"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$TAR.sha256")"
} > "$TAR.meta"
echo "wrote: $TAR"
cat "$TAR.sha256"
