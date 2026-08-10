#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/agriculture/nalt"
URL='https://lod.nal.usda.gov/rest/v1/nalt/data?format=text/turtle'
mkdir -p "$OUT"
tmp="$OUT/.nalt-full_dwn.ttl.zip.tmp"
curl -L --fail --retry 3 --retry-delay 2 -o "$tmp" "$URL"
mv "$tmp" "$OUT/nalt-full_dwn.ttl.zip"
rm -rf "$OUT/unpacked"
mkdir -p "$OUT/unpacked"
unzip -q -o "$OUT/nalt-full_dwn.ttl.zip" -d "$OUT/unpacked"
sha256sum "$OUT/nalt-full_dwn.ttl.zip" > "$OUT/nalt-full_dwn.ttl.zip.sha256"
cat > "$OUT/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest.source_manifest.v1",
  "source_id": "nal-nalt",
  "source_url": "$URL",
  "license": "CC-BY-4.0",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "local_file": "nalt-full_dwn.ttl.zip"
}
JSON
echo "updated $OUT"
