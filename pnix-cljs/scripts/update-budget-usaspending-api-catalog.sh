#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/budget/usaspending-api-catalog"
REF="${USASPENDING_REF:-master}"
LIMIT="${USASPENDING_CONTRACT_LIMIT:-180}"
mkdir -p "$OUT/raw/contracts"
python3 - "$OUT" "$REF" "$LIMIT" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]; limit=int(sys.argv[3])
def fetch(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-usaspending-ingest/1.0'})
    with urllib.request.urlopen(req,timeout=60) as r: return r.read()
tree_url=f'https://api.github.com/repos/fedspendingtransparency/usaspending-api/git/trees/{ref}?recursive=1'
tree=json.loads(fetch(tree_url).decode())
paths=[]
for item in tree.get('tree') or []:
    p=item.get('path') or ''
    if item.get('type')=='blob' and p.startswith('usaspending_api/api_contracts/contracts/v2/') and p.endswith('.md'):
        paths.append(p)
paths=sorted(paths)[:limit]
files=[]
for p in paths:
    url=f'https://raw.githubusercontent.com/fedspendingtransparency/usaspending-api/{ref}/{p}'
    data=fetch(url)
    rel='contracts/'+p.split('/contracts/v2/',1)[1].replace('/','__')
    dest=out/'raw'/rel; dest.parent.mkdir(parents=True,exist_ok=True); dest.write_bytes(data)
    files.append({'path':p,'local_path':'raw/'+rel,'url':url,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
lic_url=f'https://raw.githubusercontent.com/fedspendingtransparency/usaspending-api/{ref}/LICENSE'
try:
    lic=fetch(lic_url); (out/'raw'/'LICENSE').write_bytes(lic)
    license_file={'path':'LICENSE','url':lic_url,'bytes':len(lic),'sha256':hashlib.sha256(lic).hexdigest()}
except Exception as ex:
    license_file={'path':'LICENSE','url':lic_url,'error':str(ex)[:200]}
manifest={'schema':'pnix.source_manifest.v1','source':'USAspending API contract catalog','repo':'fedspendingtransparency/usaspending-api','ref':ref,'tree_api':tree_url,'retrieved_at':datetime.date.today().isoformat(),'policy':'contract markdown path/schema metadata only; no API calls or payload rows','files':files,'license_file':license_file,'total_contracts_available':len(paths)}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'ok':True,'out':str(out),'contracts':len(files),'ref':ref},ensure_ascii=False))
PY
