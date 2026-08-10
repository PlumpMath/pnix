#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/science/go"
URL="${GO_URL:-https://purl.obolibrary.org/obo/go/go-basic.obo}"
OUT="$OUT_DIR/go-basic.obo"
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
