#!/usr/bin/env bash
# CWL v1.2 official schema snapshot.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CWL_SCHEMA_DEST:-$ROOT/ingest/workflow/cwl-schema-catalog}"
REF="${CWL_SCHEMA_REF:-main}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, urllib.request, sys
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
UA='pnix-cwl-schema-ingest/1.0 (schema metadata only; no workflows or execution)'
FILES=['LICENSE.txt','CommonWorkflowLanguage.yml','CommandLineTool.yml','Workflow.yml','Process.yml','Operation.yml','CommandLineTool-standalone.yml']
files=[]
for name in FILES:
    url=f'https://raw.githubusercontent.com/common-workflow-language/cwl-v1.2/{REF}/{name}'
    req=urllib.request.Request(url,headers={'User-Agent':UA})
    raw=urllib.request.urlopen(req,timeout=60).read()
    rel=f'raw/{name}'
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    files.append({'relative_path':rel,'name':name,'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Common Workflow Language v1.2 schema catalog','retrieved_at':dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),'ref':REF,'source_urls':['https://github.com/common-workflow-language/cwl-v1.2'],'license':'Apache-2.0','scope':'official schema YAML files only; no prose, conformance tests, example workflows/jobs, command payloads, execution, or mirror/graph wiring','files':files}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded CWL schema catalog: files={len(files)} -> {DEST}')
PY
