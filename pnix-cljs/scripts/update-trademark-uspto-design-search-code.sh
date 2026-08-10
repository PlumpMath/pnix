#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${USPTO_DSC_DEST:-$ROOT/ingest/trademark/uspto-design-search-code}"
UA="${USPTO_DSC_USER_AGENT:-pnix-ingest/0.1 (USPTO design search code taxonomy)}"
mkdir -p "$DEST/raw"
python3 - <<'PY' "$DEST" "$UA"
import hashlib,json,pathlib,sys,time,urllib.request
root=pathlib.Path(sys.argv[1]); ua=sys.argv[2]
url='https://tmdesigncodes.uspto.gov/dscm/proxy/api/search'
body=json.dumps({'category':'*','division':'*','subdivision':'*'}).encode()
req=urllib.request.Request(url,data=body,headers={'Content-Type':'application/json','User-Agent':ua},method='POST')
with urllib.request.urlopen(req,timeout=60) as r:
    data=r.read()
(root/'raw/design-search-codes.json').write_bytes(data)
files=[{'path':'raw/design-search-codes.json','url':url,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)}]
(root/'source-receipt.json').write_text(json.dumps({'schema':'trademark.uspto_design_search_code.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / USPTO public taxonomy metadata','files':files,'excluded':['guideline prose','descriptions/search guidance','sample images','actual trademark/logo records','clearance/infringement/registrability judgments','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched USPTO design search code taxonomy bytes={len(data)} into {root}')
PY
