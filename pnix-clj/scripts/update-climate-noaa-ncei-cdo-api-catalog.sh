#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/climate/noaa-ncei-cdo-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (API catalog metadata only; contact: local)"
DOCS_URL="${NOAA_CDO_DOCS_URL:-https://www.ncei.noaa.gov/cdo-web/webservices/v2}"
curl -fsSL -A "$UA" "$DOCS_URL" -o "$OUT/webservices-v2.html"
python3 - "$OUT" "$DOCS_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'noaa-ncei-cdo-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'CDO documentation/endpoint catalog only; tokened API data payloads and observations excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
