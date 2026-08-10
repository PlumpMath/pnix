#!/usr/bin/env bash
# OpenRPC official release specification snapshot.
# Downloads official schema/spec files only. No real OpenRPC documents, endpoints, credentials, logs, or invocation.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${OPENRPC_SPEC_DEST:-$ROOT/ingest/code/openrpc-specification}"
REF="${OPENRPC_SPEC_REF:-v1.4.1}"
mkdir -p "$DEST/raw/spec/1.4"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
BASE=f'https://raw.githubusercontent.com/open-rpc/spec/{urllib.parse.quote(REF, safe="")}'
FILES=['spec/1.4/schema.json']
def get_bytes(path):
    url=f'{BASE}/{urllib.parse.quote(path, safe="/")}'
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-openrpc-spec-ingest'})
    with urllib.request.urlopen(req,timeout=60) as r:
        return url,r.read()
records=[]
for path in FILES:
    url,raw=get_bytes(path)
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    role='json_schema' if path.endswith('.json') else 'spec_doc'
    records.append({'source_path':path,'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
url,lic=get_bytes('LICENSE.md')
(DEST/'LICENSE.md').write_bytes(lic)
records.append({'source_path':'LICENSE.md','relative_path':'LICENSE.md','url':url,'sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OpenRPC official specification schema','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/open-rpc/spec','https://github.com/open-rpc/spec/releases'],'license':'Apache-2.0','scope':'official release schema structure only; no prose fields/examples/real OpenRPC documents/endpoints/credentials/request logs/response payloads/invocation/graph wiring','files':records,'selected_file_count':len(FILES)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded OpenRPC spec snapshot: ref={REF} files={len(FILES)} -> {DEST}')
PY
