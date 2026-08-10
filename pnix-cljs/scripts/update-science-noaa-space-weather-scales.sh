#!/usr/bin/env bash
# NOAA SWPC Space Weather Scales updater.
# Downloads static scale explanation HTML only. No forecast/feed/mirror/graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/noaa-space-weather-scales"
URL="${NOAA_SPACE_WEATHER_SCALES_URL:-https://www.swpc.noaa.gov/noaa-scales-explanation}"
mkdir -p "$DEST"
HTML="$DEST/noaa-scales-explanation.html"
TMP="$HTML.tmp"
curl -fsSL -A 'pnix-ingest/1.0' "$URL" -o "$TMP"
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
SIZE="$(wc -c < "$TMP" | tr -d ' ')"
mv "$TMP" "$HTML"
python3 - "$DEST/source-receipt.json" "$URL" "$SHA" "$SIZE" <<'PY'
import json,sys,datetime
out,url,sha,size=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'NOAA SWPC Space Weather Scales',
  'version':'snapshot-2026-06-19',
  'url':url,
  'sha256':sha,
  'size_bytes':int(size),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'G/R/S scale taxonomy and physical threshold metadata only; effects prose/forecast feeds/alerts/operational instructions excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated NOAA SWPC scales snapshot: $SHA $SIZE bytes"
