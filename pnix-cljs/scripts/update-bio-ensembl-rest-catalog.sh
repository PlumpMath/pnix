#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/ensembl-rest-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, pathlib, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
urls=[('rest_release','https://rest.ensembl.org/info/rest?content-type=application/json'),('software_release','https://rest.ensembl.org/info/software?content-type=application/json'),('species','https://rest.ensembl.org/info/species?content-type=application/json')]
files=[]
for kind,url in urls:
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-ensembl-rest-catalog/1.0','Accept':'application/json'})
    with urllib.request.urlopen(req,timeout=60) as r:
        data=r.read(); ctype=r.headers.get('content-type') or ''
    rel=f'raw/{kind}.json'
    (out/rel).write_bytes(data)
    files.append({'kind':kind,'url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'Ensembl REST metadata endpoints','retrieved_at':datetime.date.today().isoformat(),'policy':'release/species catalog metadata only; no gene/transcript/sequence/variant payloads','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
