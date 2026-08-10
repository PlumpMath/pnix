#!/usr/bin/env bash
# MLflow official model registry schema snapshot.
# Downloads official schema/source files only. No live registry rows, experiments, artifacts, model files, credentials, or invocation.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${MLFLOW_REGISTRY_DEST:-$ROOT/ingest/code/mlflow-registry}"
REF="${MLFLOW_REGISTRY_REF:-v3.14.0}"
mkdir -p "$DEST/raw/mlflow/protos" "$DEST/raw/mlflow/entities/model_registry"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
BASE=f'https://raw.githubusercontent.com/mlflow/mlflow/{urllib.parse.quote(REF, safe="")}'
FILES=[
 'mlflow/protos/model_registry.proto',
 'mlflow/protos/service.proto',
 'mlflow/entities/model_registry/model_version.py',
 'mlflow/entities/model_registry/registered_model.py',
]
def get_bytes(path):
    url=f'{BASE}/{urllib.parse.quote(path, safe="/")}'
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-mlflow-registry-ingest'})
    with urllib.request.urlopen(req,timeout=90) as r: return url,r.read()
records=[]
for path in FILES:
    url,raw=get_bytes(path)
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    role='proto_schema' if path.endswith('.proto') else 'registry_entity_source'
    records.append({'source_path':path,'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
url,lic=get_bytes('LICENSE.txt')
(DEST/'LICENSE.txt').write_bytes(lic)
records.append({'source_path':'LICENSE.txt','relative_path':'LICENSE.txt','url':url,'sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'MLflow official model registry schema metadata','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/mlflow/mlflow','https://github.com/mlflow/mlflow/tree/'+REF],'license':'Apache-2.0','scope':'official model registry schema/source structure only; no experiment values/tracking artifacts/model files/weights/credentials/live registry rows/invocation/graph wiring','files':records,'selected_file_count':len(FILES)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded MLflow registry snapshot: ref={REF} files={len(FILES)} -> {DEST}')
PY
