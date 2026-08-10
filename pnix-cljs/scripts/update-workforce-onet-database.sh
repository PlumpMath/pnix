#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/workforce/onet-database"
URL="${ONET_DB_URL:-https://www.onetcenter.org/dl_files/database/db_30_3_text.zip}"
mkdir -p "$OUT"
tmp="$OUT/.db_text.zip.tmp"
curl -L --fail --retry 3 --retry-delay 2 -o "$tmp" "$URL"
mv "$tmp" "$OUT/db_text.zip"
rm -rf "$OUT/unpacked"
mkdir -p "$OUT/unpacked"
unzip -q -o "$OUT/db_text.zip" -d "$OUT/unpacked"
sha256sum "$OUT/db_text.zip" > "$OUT/db_text.zip.sha256"
cat > "$OUT/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest.source_manifest.v1",
  "source_id": "onet-database",
  "source_url": "$URL",
  "license": "CC-BY-4.0",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "local_file": "db_text.zip"
}
JSON
echo "updated $OUT"
