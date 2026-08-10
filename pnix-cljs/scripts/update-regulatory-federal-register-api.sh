#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/regulatory/federal-register-api"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (schema metadata only; contact: local)"
DOCS_URL="${FEDREG_DOCS_URL:-https://www.federalregister.gov/developers/documentation/api/v1}"
DOCUMENTS_URL="${FEDREG_DOCUMENTS_URL:-https://www.federalregister.gov/api/v1/documents.json?per_page=1&order=newest}"
AGENCIES_URL="${FEDREG_AGENCIES_URL:-https://www.federalregister.gov/api/v1/agencies.json}"
curl -fsSL -A "$UA" "$DOCS_URL" -o "$OUT/api-docs.html"
curl -fsSL -A "$UA" "$DOCUMENTS_URL" -o "$OUT/documents-sample.json"
curl -fsSL -A "$UA" "$AGENCIES_URL" -o "$OUT/agencies.json"
python3 - "$OUT" "$DOCS_URL" "$DOCUMENTS_URL" "$AGENCIES_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'federal-register-api','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'endpoint and response key/type inventory only; document bodies/prose/legal advice excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
