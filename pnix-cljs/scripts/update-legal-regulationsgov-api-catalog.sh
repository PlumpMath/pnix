#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/legal/regulationsgov-api-catalog"
mkdir -p "$OUT"
DOC_URL="${REGULATIONS_GOV_DOC_URL:-https://open.gsa.gov/api/regulationsgov/}"
API_ROOT="${REGULATIONS_GOV_API_ROOT:-https://api.regulations.gov/v4/}"
curl -L --fail --max-time 45 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$DOC_URL" -o "$OUT/open-gsa-doc.html"
# Key-gated status probe only; payload not retained beyond headers/status JSON.
python3 - "$OUT" "$API_ROOT" <<'PY'
import json,pathlib,sys,urllib.request,urllib.error,datetime,hashlib
out=pathlib.Path(sys.argv[1]); api=sys.argv[2]
status={'url':api,'status':None,'content_type':None,'error':None}
try:
    r=urllib.request.urlopen(urllib.request.Request(api,headers={'User-Agent':'pnix-ingest/1.0'}),timeout=20)
    status['status']=r.status; status['content_type']=r.headers.get('content-type')
except urllib.error.HTTPError as e:
    status['status']=e.code; status['content_type']=e.headers.get('content-type'); status['error']='HTTPError'
except Exception as e:
    status['error']=type(e).__name__
(out/'api-root-status.json').write_text(json.dumps(status,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
files=[]
for p in sorted(out.glob('*')):
    if p.is_file() and p.name!='source-manifest.json':
        b=p.read_bytes(); files.append({'path':p.name,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'Regulations.gov API documentation metadata','retrieved_at':datetime.date.today().isoformat(),'doc_url':sys.argv[2] if False else None,'source_urls':[{'file':'open-gsa-doc.html','url':__import__('os').environ.get('REGULATIONS_GOV_DOC_URL','https://open.gsa.gov/api/regulationsgov/')},{'file':'api-root-status.json','url':api}],'files':files,'policy':'official docs metadata and key-gated status only; docket/comment payloads excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files),'api_status':status},ensure_ascii=False))
PY
