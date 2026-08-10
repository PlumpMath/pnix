#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/learning/lrmi-terms"
REF="${LRMI_REF:-main}"
mkdir -p "$OUT/raw"
python3 - "$OUT" "$REF" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
def fetch(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-lrmi-ingest/1.0'})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read()
tree_url=f'https://api.github.com/repos/dcmi/lrmi/git/trees/{ref}?recursive=1'
tree=json.loads(fetch(tree_url).decode('utf-8'))
paths=[]
for item in tree.get('tree') or []:
    p=item.get('path') or ''
    if item.get('type')!='blob' or not p.endswith('.ttl'):
        continue
    if p.startswith('lrmi_terms/') or (p.startswith('lrmi_vocabs/') and '/other_non_LRMI/' not in p and not p.endswith('/ex_lecture.ttl')):
        paths.append(p)
files=[]
for p in sorted(paths):
    url=f'https://raw.githubusercontent.com/dcmi/lrmi/{ref}/{p}'
    data=fetch(url)
    dest=out/'raw'/p
    dest.parent.mkdir(parents=True,exist_ok=True)
    dest.write_bytes(data)
    files.append({'path':p,'url':url,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
manifest={'schema':'pnix.source_manifest.v1','source':'LRMI Turtle terms and vocabularies','repo':'dcmi/lrmi','ref':ref,'tree_api':tree_url,'retrieved_at':datetime.date.today().isoformat(),'policy':'TTL term/vocabulary files only; examples, prose docs, JSON-LD/HTML/RDFa examples excluded','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'ttl_files':len(files),'ref':ref},ensure_ascii=False))
PY
