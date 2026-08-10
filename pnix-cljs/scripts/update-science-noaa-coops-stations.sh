#!/usr/bin/env bash
# NOAA CO-OPS station metadata updater. No observations/predictions/alerts/navigation output.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/noaa-coops-stations"
URL="${NOAA_COOPS_STATIONS_URL:-https://api.tidesandcurrents.noaa.gov/mdapi/prod/webapi/stations.json}"
mkdir -p "$DEST"
JSON_OUT="$DEST/stations.json"
TMP="$JSON_OUT.tmp"
curl -fsSL -A 'pnix-ingest/1.0' "$URL" -o "$TMP"
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
SIZE="$(wc -c < "$TMP" | tr -d ' ')"
COUNT="$(python3 - "$TMP" <<'PY'
import json,sys
print(json.load(open(sys.argv[1])).get('count', len(json.load(open(sys.argv[1])).get('stations',[]))))
PY
)"
mv "$TMP" "$JSON_OUT"
python3 - "$DEST/source-receipt.json" "$URL" "$SHA" "$SIZE" "$COUNT" <<'PY'
import json,sys,datetime
out,url,sha,size,count=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'NOAA CO-OPS station metadata',
  'version':'snapshot-2026-06-19',
  'url':url,
  'sha256':sha,
  'size_bytes':int(size),
  'row_count':int(count),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'base station registry metadata only; observations/predictions/forecasts/alerts/navigation decisions excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated NOAA CO-OPS stations snapshot: rows=$COUNT sha=$SHA bytes=$SIZE"
