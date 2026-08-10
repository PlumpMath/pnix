#!/usr/bin/env bash
# USGS Geomagnetism Observatories updater.
# Downloads static public observatory registry page only. No realtime data, alerts, or graph/mirror wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usgs-geomagnetism-observatories"
URL="${USGS_GEOMAG_OBSERVATORIES_URL:-https://www.usgs.gov/programs/geomagnetism/science/observatories}"
mkdir -p "$DEST"
HTML="$DEST/observatories.html"
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
  'source':'USGS Geomagnetism Observatories',
  'version':'snapshot-2026-06-19',
  'url':url,
  'sha256':sha,
  'size_bytes':int(size),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'kmz_download_status':'blocked_403_during_manual_probe',
  'scope':'observatory code/name/page URL metadata only; geomagnetic time-series/realtime feeds/alerts/operational decisions excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated USGS Geomagnetism Observatories snapshot: $SHA $SIZE bytes"
