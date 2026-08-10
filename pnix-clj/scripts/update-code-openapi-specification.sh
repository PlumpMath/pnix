#!/usr/bin/env bash
# OpenAPI official schema URI snapshot.
# Fetches only official schema JSON documents, not spec prose or real API definitions.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${OPENAPI_SPEC_DEST:-$ROOT/ingest/code/openapi-specification}"
mkdir -p "$DEST/raw"
python3 - "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, urllib.request, sys
DEST=pathlib.Path(sys.argv[1])
SCHEMAS=[
  ('3.0','schema','2021-09-28','https://spec.openapis.org/oas/3.0/schema/2021-09-28'),
  ('3.0','schema','2024-10-18','https://spec.openapis.org/oas/3.0/schema/2024-10-18'),
  ('3.1','schema','2022-10-07','https://spec.openapis.org/oas/3.1/schema/2022-10-07'),
  ('3.1','schema-base','2022-10-07','https://spec.openapis.org/oas/3.1/schema-base/2022-10-07'),
  ('3.1','dialect','base','https://spec.openapis.org/oas/3.1/dialect/base'),
  ('3.1','meta','base','https://spec.openapis.org/oas/3.1/meta/base'),
  ('3.2','schema','2025-09-17','https://spec.openapis.org/oas/3.2/schema/2025-09-17'),
  ('3.2','schema-base','2025-09-17','https://spec.openapis.org/oas/3.2/schema-base/2025-09-17'),
  ('3.2','dialect','2025-09-17','https://spec.openapis.org/oas/3.2/dialect/2025-09-17'),
  ('3.2','meta','2025-09-17','https://spec.openapis.org/oas/3.2/meta/2025-09-17'),
  ('3.2','schema','2025-11-23','https://spec.openapis.org/oas/3.2/schema/2025-11-23'),
  ('3.2','schema-base','2025-11-23','https://spec.openapis.org/oas/3.2/schema-base/2025-11-23'),
]
files=[]
for family,kind,version,url in SCHEMAS:
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-openapi-spec-ingest','Accept':'application/schema+json, application/json'})
    raw=urllib.request.urlopen(req,timeout=60).read()
    data=json.loads(raw.decode('utf-8'))
    rel=pathlib.Path('raw')/family/kind/(version+'.json')
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True)
    text=json.dumps(data,ensure_ascii=False,indent=2,sort_keys=True)+'\n'
    p.write_text(text,encoding='utf-8')
    files.append({'family':family,'kind':kind,'version':version,'url':url,'relative_path':str(rel),'schema_id':data.get('$id') or data.get('id'),'sha256':hashlib.sha256(text.encode()).hexdigest(),'size_bytes':len(text.encode())})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OpenAPI Specification official schemas','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/OAI/OpenAPI-Specification','https://spec.openapis.org/oas/'],'license':'Apache-2.0','scope':'official OpenAPI schema JSON documents only; no spec prose, real API documents, secrets, server URLs, logs, execution, or graph/mirror wiring','files':files}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded OpenAPI official schemas: files={len(files)} -> {DEST}')
PY
