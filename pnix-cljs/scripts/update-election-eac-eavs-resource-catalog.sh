#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/election/eac-eavs-resource-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, pathlib, urllib.request, os
out=pathlib.Path(__import__('sys').argv[1])
url=os.environ.get('EAC_EAVS_REPORTS_URL','https://www.eac.gov/research-and-data/studies-and-reports')
req=urllib.request.Request(url,headers={'User-Agent':'pnix-eac-eavs-ingest/1.0'})
with urllib.request.urlopen(req,timeout=60) as r:
    data=r.read(); ctype=r.headers.get('content-type') or ''
rel='raw/studies-and-reports.html'
(out/rel).write_bytes(data)
manifest={'schema':'pnix.source_manifest.v1','source':'EAC EAVS reports/materials page','retrieved_at':datetime.date.today().isoformat(),'policy':'resource-link catalog metadata only; no EAVS data/report payload downloads','files':[{'kind':'html_catalog','url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype}]}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'bytes':len(data)},ensure_ascii=False))
PY
