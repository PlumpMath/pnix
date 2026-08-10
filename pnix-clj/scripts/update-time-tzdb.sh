#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/time/tzdb"
URL="${TZDB_URL:-https://www.iana.org/time-zones/repository/tzdata-latest.tar.gz}"
TAR="$OUT_DIR/tzdata-latest.tar.gz"
TMP="$TAR.tmp.$$"
mkdir -p "$OUT_DIR/src"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$TAR"
shasum -a 256 "$TAR" > "$TAR.sha256"
rm -rf "$OUT_DIR/src" && mkdir -p "$OUT_DIR/src"
tar -xzf "$TAR" -C "$OUT_DIR/src" zone1970.tab zone.tab iso3166.tab version theory 2>/dev/null || tar -xzf "$TAR" -C "$OUT_DIR/src" zone1970.tab zone.tab iso3166.tab version
{
  echo "source_url=$URL"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$TAR.sha256")"
  if [ -f "$OUT_DIR/src/version" ]; then echo "tzdb_version=$(cat "$OUT_DIR/src/version")"; fi
} > "$TAR.meta"
echo "wrote: $TAR"
cat "$TAR.sha256"
