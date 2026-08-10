#!/usr/bin/env bash
# Protocol Buffers official release core .proto snapshot.
# Fetches only selected official .proto schema files. No test protos, generated code, runtime source, descriptor registry, or payloads.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${PROTOBUF_SPEC_DEST:-$ROOT/ingest/code/protobuf-specification}"
REF="${PROTOBUF_SPEC_REF:-v35.1}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
API='https://api.github.com/repos/protocolbuffers/protobuf/contents'
RAW=f'https://raw.githubusercontent.com/protocolbuffers/protobuf/{REF}'
def get_json(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-protobuf-spec-ingest','Accept':'application/vnd.github+json'})
    return json.load(urllib.request.urlopen(req,timeout=60))
def get_bytes(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-protobuf-spec-ingest'})
    return urllib.request.urlopen(req,timeout=60).read()
paths=[]
for item in get_json(f'{API}/src/google/protobuf?ref={REF}'):
    n=item.get('name','')
    if item.get('type')=='file' and n.endswith('.proto') and '_test' not in n and not n.endswith('_unittest.proto'):
        paths.append('src/google/protobuf/'+n)
# compiler plugin schema is part of public protoc plugin contract.
for item in get_json(f'{API}/src/google/protobuf/compiler?ref={REF}'):
    n=item.get('name','')
    if item.get('type')=='file' and n == 'plugin.proto':
        paths.append('src/google/protobuf/compiler/'+n)
paths=sorted(dict.fromkeys(paths))
records=[]
for path in paths:
    raw=get_bytes(f'{RAW}/{path}')
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    records.append({'source_path':path,'relative_path':str(rel),'url':f'{RAW}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
lic=get_bytes(f'{RAW}/LICENSE')
(DEST/'LICENSE').write_bytes(lic)
records.append({'source_path':'LICENSE','relative_path':'LICENSE','url':f'{RAW}/LICENSE','sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Protocol Buffers official core .proto schema files','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/protocolbuffers/protobuf','https://github.com/protocolbuffers/protobuf/releases'],'license':'BSD-style permissive license','scope':'official core .proto schema files only; no tests/generated code/runtime source/production descriptor sets/schema registries/customer protos/payloads/execution/graph wiring','files':records,'proto_file_count':len(paths)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded Protobuf core proto snapshot: ref={REF} proto_files={len(paths)} -> {DEST}')
PY
