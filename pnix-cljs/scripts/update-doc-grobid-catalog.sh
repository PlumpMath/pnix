#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/doc/grobid-catalog"
REF="${GROBID_REF:-master}"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/kermitt2/grobid/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "https://raw.githubusercontent.com/kermitt2/grobid/$REF/LICENSE"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for name in ['tree.json','LICENSE']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'grobid-catalog','source_name':'GROBID class catalog','license_id':'Apache-2.0','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/kermitt2/grobid','https://api.github.com/repos/kermitt2/grobid/git/trees/'+ref+'?recursive=1'],'files':files,'policy':'Git tree path metadata only. Exclude Java source bodies, docs prose, examples, tests, model/data files, parsed scholarly text, runtime extraction, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} files={len(files)}')
PY
