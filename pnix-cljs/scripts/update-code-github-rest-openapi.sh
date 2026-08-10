#!/usr/bin/env bash
# GitHub REST API official OpenAPI snapshot.
# Downloads official OpenAPI JSON only. No API calls, tokens, private data, logs, payload samples, or invocation.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${GITHUB_REST_OPENAPI_DEST:-$ROOT/ingest/code/github-rest-openapi}"
REF="${GITHUB_REST_OPENAPI_REF:-v2.1.0}"
mkdir -p "$DEST/raw/descriptions/api.github.com"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
BASE=f'https://raw.githubusercontent.com/github/rest-api-description/{urllib.parse.quote(REF, safe="")}'
FILES=['descriptions/api.github.com/api.github.com.json']
def get_bytes(path):
    url=f'{BASE}/{urllib.parse.quote(path, safe="/")}'
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-github-rest-openapi-ingest'})
    with urllib.request.urlopen(req,timeout=120) as r:
        return url,r.read()
records=[]
for path in FILES:
    url,raw=get_bytes(path)
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    records.append({'source_path':path,'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'openapi_json'})
url,lic=get_bytes('LICENSE.md')
(DEST/'LICENSE.md').write_bytes(lic)
records.append({'source_path':'LICENSE.md','relative_path':'LICENSE.md','url':url,'sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'GitHub REST API official OpenAPI description','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/github/rest-api-description','https://github.com/github/rest-api-description/tree/'+REF],'license':'MIT','scope':'official OpenAPI structure only; no descriptions/examples/API calls/tokens/private repo data/diff bodies/secrets/request logs/response payloads/execution/graph wiring','files':records,'selected_file_count':len(FILES)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded GitHub REST OpenAPI snapshot: ref={REF} files={len(FILES)} -> {DEST}')
PY
