#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/doc-layout/pubtables1m-schema-catalog"
REF="${PUBTABLES1M_REF:-main}"
mkdir -p "$OUT/raw"
python3 - "$OUT" "$REF" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
def fetch(url):
    req=urllib.request.Request(url,headers={"User-Agent":"pnix-pubtables1m-ingest/1.0"})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read()
sources=[
    {'id':'github_readme','url':f'https://raw.githubusercontent.com/microsoft/table-transformer/{ref}/README.md','path':'github-README.md'},
    {'id':'github_license','url':f'https://raw.githubusercontent.com/microsoft/table-transformer/{ref}/LICENSE','path':'github-LICENSE'},
    {'id':'hf_detection_config','url':'https://huggingface.co/microsoft/table-transformer-detection/raw/main/config.json','path':'hf-detection-config.json'},
    {'id':'hf_structure_config','url':'https://huggingface.co/microsoft/table-transformer-structure-recognition/raw/main/config.json','path':'hf-structure-config.json'}
]
files=[]
for s in sources:
    try:
        data=fetch(s['url'])
        dest=out/'raw'/s['path']
        dest.parent.mkdir(parents=True,exist_ok=True)
        dest.write_bytes(data)
        files.append({**s,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'status':'ok'})
    except Exception as ex:
        files.append({**s,'status':'error','error':str(ex)[:240]})
manifest={'schema':'pnix.source_manifest.v1','source':'PubTables-1M / Table Transformer schema/catalog metadata','repo':'microsoft/table-transformer','ref':ref,'retrieved_at':datetime.date.today().isoformat(),'policy':'README/license/model-config label maps only; no dataset archives/images/XML/JSON/model weights downloaded','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':files},ensure_ascii=False))
PY
