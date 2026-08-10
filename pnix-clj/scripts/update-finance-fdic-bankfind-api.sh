#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/finance/fdic-bankfind-api"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (schema metadata only; contact: local)"
DOCS_URL="${FDIC_BANKFIND_DOCS_URL:-https://banks.data.fdic.gov/docs/}"
INSTITUTIONS_URL="${FDIC_BANKFIND_INSTITUTIONS_URL:-https://banks.data.fdic.gov/api/institutions?limit=1&format=json&download=false}"
LOCATIONS_URL="${FDIC_BANKFIND_LOCATIONS_URL:-https://banks.data.fdic.gov/api/locations?limit=1&format=json&download=false}"
SUMMARY_URL="${FDIC_BANKFIND_SUMMARY_URL:-https://banks.data.fdic.gov/api/summary?limit=1&format=json&download=false}"
curl -fsSL -A "$UA" "$DOCS_URL" -o "$OUT/docs.html"
curl -fsSL -A "$UA" "$INSTITUTIONS_URL" -o "$OUT/institutions-sample.json"
curl -fsSL -A "$UA" "$LOCATIONS_URL" -o "$OUT/locations-sample.json"
curl -fsSL -A "$UA" "$SUMMARY_URL" -o "$OUT/summary-sample.json"
python3 - "$OUT" "$DOCS_URL" "$INSTITUTIONS_URL" "$LOCATIONS_URL" "$SUMMARY_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'fdic-bankfind-api','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'endpoint and response field/type inventory only; institution payload values/addresses/amounts/advice excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
