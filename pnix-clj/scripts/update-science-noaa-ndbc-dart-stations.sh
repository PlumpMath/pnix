#!/usr/bin/env bash
# NOAA NDBC DART station metadata updater. No observations/alerts/forecasts/tsunami guidance.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/noaa-ndbc-dart-stations"
URL="${NOAA_NDBC_ACTIVE_STATIONS_URL:-https://www.ndbc.noaa.gov/activestations.xml}"
mkdir -p "$DEST"
XML="$DEST/activestations.xml"
TMP="$XML.tmp"
curl -fsSL -A 'pnix-ingest/1.0' "$URL" -o "$TMP"
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
SIZE="$(wc -c < "$TMP" | tr -d ' ')"
read TOTAL DART CREATED < <(python3 - "$TMP" <<'PY'
import sys, xml.etree.ElementTree as ET
root=ET.parse(sys.argv[1]).getroot()
total=len(root.findall('station'))
dart=sum(1 for s in root.findall('station') if s.get('dart')=='y')
print(total, dart, root.get('created') or '')
PY
)
mv "$TMP" "$XML"
python3 - "$DEST/source-receipt.json" "$URL" "$SHA" "$SIZE" "$TOTAL" "$DART" "$CREATED" <<'PY'
import json,sys,datetime
out,url,sha,size,total,dart,created=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'NOAA NDBC DART station metadata',
  'version':'snapshot-2026-06-19',
  'url':url,
  'sha256':sha,
  'size_bytes':int(size),
  'total_station_count':int(total),
  'dart_station_count':int(dart),
  'source_created':created,
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'NDBC active station metadata filtered to dart=y only; observations/alerts/forecasts/tsunami guidance excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated NOAA NDBC DART stations snapshot: total=$TOTAL dart=$DART sha=$SHA bytes=$SIZE"
