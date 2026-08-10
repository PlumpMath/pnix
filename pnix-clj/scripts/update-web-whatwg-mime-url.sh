#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/web/whatwg-mime-url"
mkdir -p "$OUT"
URL_REF="${WHATWG_URL_REF:-main}"
MIME_REF="${WHATWG_MIME_REF:-main}"
fetch() { curl -L --fail --max-time 45 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$1" -o "$2"; }
fetch "https://raw.githubusercontent.com/whatwg/url/$URL_REF/url.bs" "$OUT/url.bs"
fetch "https://raw.githubusercontent.com/whatwg/mimesniff/$MIME_REF/mimesniff.bs" "$OUT/mimesniff.bs"
fetch "https://api.github.com/repos/whatwg/url/commits/$URL_REF" "$OUT/url-commit.json"
fetch "https://api.github.com/repos/whatwg/mimesniff/commits/$MIME_REF" "$OUT/mimesniff-commit.json"
python3 - "$OUT" "$URL_REF" "$MIME_REF" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); url_ref=sys.argv[2]; mime_ref=sys.argv[3]
files=[]
for p in sorted(out.glob('*')):
    if p.is_file() and p.name!='source-manifest.json':
        b=p.read_bytes(); files.append({'path':p.name,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
commits={}
for name in ['url','mimesniff']:
    p=out/(name+'-commit.json')
    if p.exists():
        j=json.loads(p.read_text())
        commits[name]={'sha':j.get('sha'), 'date':((j.get('commit') or {}).get('committer') or {}).get('date')}
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'WHATWG URL and MIME Sniffing structural metadata','retrieved_at':datetime.date.today().isoformat(),'refs':{'url':url_ref,'mimesniff':mime_ref},'commits':commits,'files':files,'policy':'Bikeshed source structural identifiers only; prose/examples/runtime execution excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
