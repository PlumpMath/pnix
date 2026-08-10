#!/usr/bin/env bash
# USGS NWIS Site Service metadata updater. No observations/time-series/water-quality/safety judgments.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usgs-nwis-sites"
STATE="${USGS_NWIS_STATECD:-CA}"
SITE_TYPE="${USGS_NWIS_SITETYPE:-ST}"
PARAM="${USGS_NWIS_PARAMETERCD:-00060}"
STATUS="${USGS_NWIS_SITESTATUS:-active}"
mkdir -p "$DEST"
OUT="$DEST/nwis-sites.rdb"
python3 - "$OUT" "$DEST/source-receipt.json" "$STATE" "$SITE_TYPE" "$PARAM" "$STATUS" <<'PY'
import sys, urllib.parse, urllib.request, hashlib, json, datetime
out,receipt_path,state,site_type,param,status=sys.argv[1:]
base='https://waterservices.usgs.gov/nwis/site/'
params={'format':'rdb','stateCd':state,'siteStatus':status,'siteType':site_type,'parameterCd':param}
url=base+'?'+urllib.parse.urlencode(params)
req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0'})
with urllib.request.urlopen(req,timeout=120) as r:
    data=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
open(out,'wb').write(data)
text=data.decode('utf-8','replace')
non=[l for l in text.splitlines() if l and not l.startswith('#')]
row_count=max(0,len(non)-2)
sha=hashlib.sha256(data).hexdigest()
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'USGS NWIS Site Service monitoring-location metadata',
  'version':f'snapshot-2026-06-19-state-{state}-siteType-{site_type}-parameter-{param}',
  'url':url,
  'final_url':final,
  'sha256':sha,
  'size_bytes':len(data),
  'content_type':ctype,
  'row_count':row_count,
  'query':params,
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'bounded NWIS site metadata only; observations/time-series/water-quality/private wells/water-rights/safety judgments excluded'
}, open(receipt_path,'w'), indent=2, ensure_ascii=False)
print(f'updated USGS NWIS sites snapshot: rows={row_count} sha={sha} bytes={len(data)} query={params}')
PY
