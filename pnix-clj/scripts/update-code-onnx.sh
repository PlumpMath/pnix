#!/usr/bin/env bash
# ONNX official release IR/operator spec snapshot.
# Downloads official proto/docs only. No model files, weights, datasets, runtime execution, generated code, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ONNX_DEST:-$ROOT/ingest/code/onnx}"
REF="${ONNX_REF:-v1.22.0}"
mkdir -p "$DEST/raw/onnx" "$DEST/raw/docs"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
BASE=f'https://raw.githubusercontent.com/onnx/onnx/{urllib.parse.quote(REF, safe="")}'
FILES=['onnx/onnx.proto','onnx/onnx-ml.proto','docs/Operators.md','docs/IR.md','docs/Versioning.md']
def get_bytes(path):
    url=f'{BASE}/{urllib.parse.quote(path, safe="/")}'
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-onnx-ingest'})
    with urllib.request.urlopen(req,timeout=120) as r: return url,r.read()
records=[]
for path in FILES:
    url,raw=get_bytes(path)
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    role='proto_schema' if path.endswith('.proto') else 'spec_doc'
    records.append({'source_path':path,'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
url,lic=get_bytes('LICENSE')
(DEST/'LICENSE').write_bytes(lic)
records.append({'source_path':'LICENSE','relative_path':'LICENSE','url':url,'sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'ONNX official IR and operator specification metadata','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/onnx/onnx','https://github.com/onnx/onnx/tree/'+REF],'license':'Apache-2.0','scope':'official proto/operator/IR structural metadata only; no prose bodies/examples/model files/weights/datasets/runtime execution/generated code/graph wiring','files':records,'selected_file_count':len(FILES)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded ONNX snapshot: ref={REF} files={len(FILES)} -> {DEST}')
PY
