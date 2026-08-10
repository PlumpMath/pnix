#!/usr/bin/env bash
# gRPC official core protocol spec snapshot.
# Downloads selected official docs only. No server reflection, live endpoints, credentials, payloads, or invocation.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${GRPC_SPEC_DEST:-$ROOT/ingest/code/grpc-specification}"
REF="${GRPC_SPEC_REF:-v1.81.1}"
mkdir -p "$DEST/raw/doc"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
RAW=f'https://raw.githubusercontent.com/grpc/grpc/{REF}'
DOCS=[
    'doc/PROTOCOL-HTTP2.md',
    'doc/PROTOCOL-WEB.md',
    'doc/health-checking.md',
    'doc/http-grpc-status-mapping.md',
]
def get_bytes(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-grpc-spec-ingest'})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read()
records=[]
for path in DOCS:
    raw=get_bytes(f'{RAW}/{path}')
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    records.append({'source_path':path,'relative_path':str(rel),'url':f'{RAW}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'core_protocol_doc'})
lic=get_bytes(f'{RAW}/LICENSE')
(DEST/'LICENSE').write_bytes(lic)
records.append({'source_path':'LICENSE','relative_path':'LICENSE','url':f'{RAW}/LICENSE','sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'gRPC official core protocol specification documents','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/grpc/grpc','https://github.com/grpc/grpc/releases','https://github.com/grpc/grpc/tree/master/doc'],'license':'Apache-2.0','scope':'official core protocol docs only; no full prose bodies in generated output; no server reflection/prod schema export/customer descriptors/payload logs/live endpoints/credentials/invocation/graph wiring','files':records,'doc_count':len(DOCS)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded gRPC core protocol snapshot: ref={REF} docs={len(DOCS)} -> {DEST}')
PY
