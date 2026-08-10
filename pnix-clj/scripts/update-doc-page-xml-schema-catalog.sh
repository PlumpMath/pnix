#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/doc-layout/page-xml-schema-catalog"
REF="${PAGE_XML_REF:-master}"
LIMIT="${PAGE_XML_XSD_LIMIT:-120}"
mkdir -p "$OUT/raw"
python3 - "$OUT" "$REF" "$LIMIT" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]; limit=int(sys.argv[3])
api=f"https://api.github.com/repos/PRImA-Research-Lab/PAGE-XML/git/trees/{ref}?recursive=1"
def fetch(url):
    req=urllib.request.Request(url,headers={"User-Agent":"pnix-page-xml-ingest/1.0"})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read()
tree=json.loads(fetch(api).decode('utf-8'))
paths=[]
for item in tree.get('tree') or []:
    p=item.get('path') or ''
    if item.get('type')=='blob' and p.lower().endswith('.xsd'):
        if '/example' in p.lower() or '/test' in p.lower():
            continue
        paths.append(p)
paths=sorted(paths)[:limit]
files=[]
for p in paths:
    url=f"https://raw.githubusercontent.com/PRImA-Research-Lab/PAGE-XML/{ref}/{p}"
    data=fetch(url)
    dest=out/'raw'/p
    dest.parent.mkdir(parents=True,exist_ok=True)
    dest.write_bytes(data)
    files.append({'path':p,'url':url,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
manifest={'schema':'pnix.source_manifest.v1','source':'PRImA PAGE-XML schema catalog','repo':'PRImA-Research-Lab/PAGE-XML','ref':ref,'tree_api':api,'retrieved_at':datetime.date.today().isoformat(),'policy':'XSD files only; docs/examples/instance XML/OCR text/image payloads excluded','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'xsd_files':len(files),'ref':ref},ensure_ascii=False))
PY
