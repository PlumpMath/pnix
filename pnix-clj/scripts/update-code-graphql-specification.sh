#!/usr/bin/env bash
# GraphQL official release specification snapshot.
# Downloads selected official spec files only. No introspection export, prod schemas, request logs, endpoints, credentials, or execution.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${GRAPHQL_SPEC_DEST:-$ROOT/ingest/code/graphql-specification}"
REF="${GRAPHQL_SPEC_REF:-September2025}"
mkdir -p "$DEST/raw/spec"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
BASE=f'https://raw.githubusercontent.com/graphql/graphql-spec/{urllib.parse.quote(REF, safe="")}'
FILES=[
    'spec/GraphQL.md',
    'spec/Section 2 -- Language.md',
    'spec/Section 3 -- Type System.md',
    'spec/Section 5 -- Validation.md',
    'spec/Appendix B -- Notation Conventions.md',
    'spec/Appendix C -- Grammar Summary.md',
    'spec/Appendix D -- Specified Definitions.md',
    'spec/metadata.json',
]
def get_bytes(path):
    url=f'{BASE}/{urllib.parse.quote(path, safe="/")}'
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-graphql-spec-ingest'})
    with urllib.request.urlopen(req,timeout=60) as r:
        return url,r.read()
records=[]
for path in FILES:
    url,raw=get_bytes(path)
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    role='metadata' if path.endswith('.json') else 'language_type_validation_spec_doc'
    records.append({'source_path':path,'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
url,lic=get_bytes('LICENSE.md')
(DEST/'LICENSE.md').write_bytes(lic)
records.append({'source_path':'LICENSE.md','relative_path':'LICENSE.md','url':url,'sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'GraphQL official specification documents','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/graphql/graphql-spec','https://github.com/graphql/graphql-spec/releases'],'license':'OWFa-1.0 for specifications; MIT for source code; CC0 for data sets','scope':'official release language/type-system/validation structural metadata only; no prose bodies/examples/introspection schema export/prod schemas/request logs/endpoints/credentials/execution/invocation/graph wiring','files':records,'selected_file_count':len(FILES)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded GraphQL spec snapshot: ref={REF} files={len(FILES)} -> {DEST}')
PY
