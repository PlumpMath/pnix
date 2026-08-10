#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/ml/torchmetrics-catalog"
REF="${TORCHMETRICS_REF:-master}"
mkdir -p "$DST/files"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/Lightning-AI/torchmetrics/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "https://raw.githubusercontent.com/Lightning-AI/torchmetrics/$REF/LICENSE"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime, urllib.request
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
tree=json.loads((root/'tree.json').read_text())
paths=sorted(p['path'] for p in tree.get('tree',[]) if p.get('type')=='blob' and p.get('path','').startswith('src/torchmetrics/') and p.get('path','').endswith('.py') and '/tests/' not in p.get('path','') and not p.get('path','').endswith('__about__.py') and '/_deprecated' not in p.get('path',''))
for path in paths:
    out=root/'files'/path
    out.parent.mkdir(parents=True, exist_ok=True)
    url=f'https://raw.githubusercontent.com/Lightning-AI/torchmetrics/{ref}/{path}'
    with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0'}),timeout=30) as r:
        out.write_bytes(r.read())
files=[]
for rel in ['tree.json','LICENSE']+[str(pathlib.Path('files')/p) for p in paths]:
    p=root/rel; b=p.read_bytes(); files.append({'path':rel,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'torchmetrics-catalog','source_name':'TorchMetrics catalog','license_id':'Apache-2.0','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/Lightning-AI/torchmetrics','https://api.github.com/repos/Lightning-AI/torchmetrics/git/trees/'+ref+'?recursive=1'],'metric_py_files':paths,'files':files,'policy':'Symbol-level metric metadata only. Exclude source bodies, docstrings, examples, tests, datasets, eval results, execution, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} metric_py_files={len(paths)}')
PY
