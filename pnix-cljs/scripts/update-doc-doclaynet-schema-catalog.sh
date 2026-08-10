#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/doc-layout/doclaynet-schema-catalog"
REF="${DOCLAYNET_REF:-main}"
mkdir -p "$OUT/raw"
python3 - "$OUT" "$REF" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
def fetch(url):
    req=urllib.request.Request(url,headers={"User-Agent":"pnix-doclaynet-ingest/1.0"})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read()
sources=[
    {'id':'github_readme','url':f'https://raw.githubusercontent.com/DS4SD/DocLayNet/{ref}/README.md','path':'github-README.md'},
    {'id':'github_license','url':f'https://raw.githubusercontent.com/DS4SD/DocLayNet/{ref}/LICENSE','path':'github-LICENSE'},
    {'id':'hf_card','url':'https://huggingface.co/datasets/docling-project/DocLayNet/raw/main/README.md','path':'huggingface-README.md'}
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
manifest={'schema':'pnix.source_manifest.v1','source':'DocLayNet schema/catalog metadata','repo':'DS4SD/DocLayNet','ref':ref,'retrieved_at':datetime.date.today().isoformat(),'policy':'README/license/card only; no dataset archives/images/PDFs/COCO annotations downloaded','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':files},ensure_ascii=False))
PY
