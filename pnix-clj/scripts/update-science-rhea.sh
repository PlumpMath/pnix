#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/science/rhea"
URL="${RHEA_URL:-https://ftp.expasy.org/databases/rhea/tsv/rhea-tsv.tar.gz}"
TAR="$OUT_DIR/rhea-tsv.tar.gz"
TMP="$TAR.tmp.$$"
mkdir -p "$OUT_DIR/tsv"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$TAR"
shasum -a 256 "$TAR" > "$TAR.sha256"
rm -f "$OUT_DIR/tsv"/*.tsv "$OUT_DIR/tsv"/*.txt
tar -xzf "$TAR" -C "$OUT_DIR/tsv"
{
  echo "source_url=$URL"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$TAR.sha256")"
} > "$TAR.meta"
echo "wrote: $TAR"
cat "$TAR.sha256"
