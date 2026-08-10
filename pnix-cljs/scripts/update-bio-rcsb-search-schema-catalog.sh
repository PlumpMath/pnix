#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/rcsb-search-schema-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, os, pathlib, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
urls=[
 ('entry_schema', os.environ.get('RCSB_SEARCH_SCHEMA_URL','https://search.rcsb.org/rcsbsearch/v2/metadata/schema')),
 ('chemical_schema', os.environ.get('RCSB_CHEMICAL_SCHEMA_URL','https://search.rcsb.org/rcsbsearch/v2/metadata/chemical/schema')),
]
files=[]
for kind,url in urls:
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-rcsb-search-schema/1.0'})
    with urllib.request.urlopen(req,timeout=60) as r:
        data=r.read(); ctype=r.headers.get('content-type') or ''
    rel=f'raw/{kind}.json'
    (out/rel).write_bytes(data)
    files.append({'kind':kind,'url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'RCSB Search API metadata schemas','retrieved_at':datetime.date.today().isoformat(),'policy':'schema/category/property metadata only; no structure/search payload rows','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
