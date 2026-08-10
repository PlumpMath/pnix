#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/legal/congressgov-api-catalog"
mkdir -p "$OUT/Documentation"
REF="${CONGRESSGOV_API_REF:-main}"
API="https://api.github.com/repos/LibraryOfCongress/api.congress.gov"
RAW="https://raw.githubusercontent.com/LibraryOfCongress/api.congress.gov/$REF"
fetch() { curl -L --fail --max-time 45 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$1" -o "$2"; }
fetch "$API/commits/$REF" "$OUT/commit.json"
fetch "$API/contents/Documentation?ref=$REF" "$OUT/documentation-contents.json"
fetch "$RAW/README.md" "$OUT/README.md"
fetch "$RAW/Documentation/swagger.json" "$OUT/Documentation/swagger.json"
fetch "$RAW/Documentation/swagger.yaml" "$OUT/Documentation/swagger.yaml"
python3 - "$OUT/documentation-contents.json" "$OUT/Documentation" <<'PY'
import json,pathlib,sys,urllib.request
arr=json.loads(pathlib.Path(sys.argv[1]).read_text())
out=pathlib.Path(sys.argv[2])
for x in arr:
    if x.get('type')=='file' and x.get('name','').endswith('Endpoint.md'):
        url=x.get('download_url')
        if url:
            data=urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0'}),timeout=45).read()
            (out/x['name']).write_bytes(data)
print(sum(1 for x in out.glob('*Endpoint.md')))
PY
python3 - "$OUT" "$REF" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for p in sorted(out.rglob('*')):
    if p.is_file() and p.name!='source-manifest.json':
        b=p.read_bytes(); files.append({'path':str(p.relative_to(out)),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
commit={}
p=out/'commit.json'
if p.exists():
    j=json.loads(p.read_text()); commit={'sha':j.get('sha'), 'date':((j.get('commit') or {}).get('committer') or {}).get('date')}
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'Congress.gov API documentation metadata','retrieved_at':datetime.date.today().isoformat(),'ref':ref,'commit':commit,'files':files,'policy':'official API documentation metadata only; live payload calls, prose bodies, examples, API keys excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
