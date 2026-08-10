#!/usr/bin/env bash
# USGS Earthquake Catalog / ComCat historical metadata updater.
# Defaults to completed historical interval and M>=6.0. No realtime alerts, forecasts, PAGER/ShakeMap, or response guidance.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usgs-earthquake-comcat"
START="${USGS_COMCAT_START:-2025-01-01}"
END="${USGS_COMCAT_END:-2026-01-01}"
MINMAG="${USGS_COMCAT_MINMAG:-6.0}"
LIMIT="${USGS_COMCAT_LIMIT:-20000}"
mkdir -p "$DEST"
OUT="$DEST/comcat-events.geojson"
python3 - "$OUT" "$DEST/source-receipt.json" "$START" "$END" "$MINMAG" "$LIMIT" <<'PY'
import sys, urllib.parse, urllib.request, hashlib, json, datetime
out,receipt_path,start,end,minmag,limit=sys.argv[1:]
base='https://earthquake.usgs.gov/fdsnws/event/1/query'
params={'format':'geojson','starttime':start,'endtime':end,'minmagnitude':minmag,'orderby':'time-asc','limit':limit}
url=base+'?'+urllib.parse.urlencode(params)
req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/1.0'})
with urllib.request.urlopen(req, timeout=120) as r:
    data=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
open(out,'wb').write(data)
sha=hashlib.sha256(data).hexdigest()
obj=json.loads(data.decode('utf-8','replace'))
count=len(obj.get('features',[]))
meta=obj.get('metadata',{})
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'USGS Earthquake Catalog / ComCat historical event metadata',
  'version':f'historical-{start}-to-{end}-m{minmag}',
  'url':url,
  'final_url':final,
  'sha256':sha,
  'size_bytes':len(data),
  'content_type':ctype,
  'row_count':count,
  'query':params,
  'api':meta.get('api'),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'bounded historical event metadata only; alerts/PAGER/ShakeMap/felt reports/tsunami warning/forecast/response guidance excluded'
}, open(receipt_path,'w'), indent=2, ensure_ascii=False)
print(f'updated USGS ComCat snapshot: rows={count} sha={sha} bytes={len(data)} query={params}')
PY
