#!/usr/bin/env bash
# AsyncAPI official schema repo snapshot.
# Fetches bounded schema JSON files and binding directory catalog only. No real AsyncAPI documents.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ASYNCAPI_SPEC_DEST:-$ROOT/ingest/code/asyncapi-specification}"
REF="${ASYNCAPI_SPEC_REF:-v6.11.1}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, urllib.request, sys
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
RAW_BASE=f'https://raw.githubusercontent.com/asyncapi/spec-json-schemas/{REF}'
API_BASE=f'https://api.github.com/repos/asyncapi/spec-json-schemas/contents'
FILES=['schemas/3.1.0.json','schemas/3.0.0.json','schemas/all.schema-store.json','common/avroSchema_v1.json','common/openapiSchema_3_0.json','LICENSE','NOTICE','package.json']
def get(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-asyncapi-spec-ingest'})
    return urllib.request.urlopen(req,timeout=60).read()
records=[]
for path in FILES:
    raw=get(f'{RAW_BASE}/{path}')
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True)
    p.write_bytes(raw)
    records.append({'relative_path':str(rel),'source_path':path,'url':f'{RAW_BASE}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'schema_json' if path.endswith('.json') else 'license_or_package'})
# Binding catalog: directory names and version directories only; no binding schemas downloaded here.
binding_catalog=[]
req=urllib.request.Request(f'{API_BASE}/bindings?ref={REF}',headers={'User-Agent':'pnix-asyncapi-spec-ingest'})
bindings=json.load(urllib.request.urlopen(req,timeout=60))
for b in bindings:
    if b.get('type')!='dir': continue
    breq=urllib.request.Request(f'{API_BASE}/bindings/{b["name"]}?ref={REF}',headers={'User-Agent':'pnix-asyncapi-spec-ingest'})
    versions=json.load(urllib.request.urlopen(breq,timeout=60))
    binding_catalog.append({'binding':b['name'],'versions':[v['name'] for v in versions if v.get('type')=='dir']})
(DEST/'binding-catalog.json').write_text(json.dumps(binding_catalog,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
records.append({'relative_path':'binding-catalog.json','source_path':'bindings/*','url':f'{API_BASE}/bindings?ref={REF}','sha256':hashlib.sha256((DEST/'binding-catalog.json').read_bytes()).hexdigest(),'size_bytes':(DEST/'binding-catalog.json').stat().st_size,'role':'binding_catalog'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'AsyncAPI official spec-json-schemas','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/asyncapi/spec','https://github.com/asyncapi/spec-json-schemas'],'license':'Apache-2.0','scope':'bounded official schema JSON + binding directory catalog only; no spec prose, real AsyncAPI documents, broker credentials, event logs, execution, or graph/mirror wiring','files':records,'binding_catalog_count':len(binding_catalog)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded AsyncAPI schema snapshot: ref={REF} files={len(records)} bindings={len(binding_catalog)} -> {DEST}')
PY
