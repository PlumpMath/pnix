#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/reactome-contentservice-api-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, os, pathlib, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
url=os.environ.get('REACTOME_OPENAPI_URL','https://reactome.org/ContentService/v3/api-docs')
req=urllib.request.Request(url,headers={'User-Agent':'pnix-reactome-api-catalog/1.0'})
with urllib.request.urlopen(req,timeout=60) as r:
    data=r.read(); ctype=r.headers.get('content-type') or ''
rel='raw/contentservice-openapi.json'
(out/rel).write_bytes(data)
manifest={'schema':'pnix.source_manifest.v1','source':'Reactome ContentService OpenAPI','retrieved_at':datetime.date.today().isoformat(),'policy':'OpenAPI endpoint/schema-key metadata only; no pathway/reaction/analysis payloads','files':[{'kind':'openapi_json','url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype}]}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'bytes':len(data)},ensure_ascii=False))
PY
