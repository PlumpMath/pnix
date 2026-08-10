#!/usr/bin/env bash
# USGS Volcano Alert Level System updater. Static taxonomy only; no live notices/events/response guidance.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usgs-volcano-alert-levels"
URL="${USGS_VOLCANO_ALERT_LEVELS_URL:-https://www.usgs.gov/programs/VHP/alert-level-system}"
mkdir -p "$DEST"
HTML="$DEST/alert-level-system.html"
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
  'source':'USGS Volcano Alert Level System',
  'version':'snapshot-2026-06-19',
  'url':url,
  'sha256':sha,
  'size_bytes':int(size),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'static volcano alert-level and aviation-color-code taxonomy only; live notices/events/prose guidance excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated USGS Volcano Alert Level System snapshot: $SHA $SIZE bytes"
