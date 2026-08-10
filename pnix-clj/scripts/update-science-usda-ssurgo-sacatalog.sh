#!/usr/bin/env bash
# USDA NRCS Soil Data Access / SSURGO sacatalog updater. Survey-area catalog metadata only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usda-ssurgo-sacatalog"
mkdir -p "$DEST"
OUT="$DEST/sacatalog.json"
python3 - "$OUT" "$DEST/source-receipt.json" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.parse, urllib.request
out, receipt_path = map(pathlib.Path, sys.argv[1:])
endpoint='https://sdmdataaccess.sc.egov.usda.gov/Tabular/post.rest'
query='SELECT areasymbol, areaname, saversion, saverest FROM sacatalog ORDER BY areasymbol'
params={'query':query,'format':'JSON'}
data=urllib.parse.urlencode(params).encode()
req=urllib.request.Request(endpoint,data=data,headers={'User-Agent':'pnix-ingest/1.0 (SSURGO sacatalog metadata only)'})
with urllib.request.urlopen(req,timeout=120) as r:
    raw=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
out.write_bytes(raw)
obj=json.loads(raw.decode('utf-8'))
rows=obj.get('Table') or []
sha=hashlib.sha256(raw).hexdigest()
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'USDA NRCS Soil Data Access / SSURGO sacatalog survey-area catalog',
  'version':'snapshot-2026-06-19-sacatalog',
  'url':endpoint,
  'final_url':final,
  'query':query,
  'sha256':sha,
  'size_bytes':len(raw),
  'content_type':ctype,
  'row_count':len(rows),
  'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'license':'US Government public domain',
  'scope':'sacatalog survey-area catalog metadata fields only; FGDC XML/prose, mapunit/component/horizon properties, spatial geometry, interpretations/ratings, and suitability judgments excluded'
}
receipt_path.write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated USDA SSURGO sacatalog: rows={len(rows)} sha={sha} bytes={len(raw)}')
PY
