#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/postgresql-sql-plan"
mkdir -p "$OUT/src/backend/parser" "$OUT/src/include/nodes"
REF="${POSTGRES_REF:-master}"
TAG_PAGES="${POSTGRES_TAG_PAGES:-20}"
API_ROOT="${POSTGRES_API_ROOT:-https://api.github.com/repos/postgres/postgres}"
RAW_ROOT="https://raw.githubusercontent.com/postgres/postgres/$REF"
fetch() { curl -L --fail --max-time 45 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$1" -o "$2"; }
fetch "$RAW_ROOT/src/backend/parser/gram.y" "$OUT/src/backend/parser/gram.y"
for f in nodes.h plannodes.h primnodes.h parsenodes.h; do fetch "$RAW_ROOT/src/include/nodes/$f" "$OUT/src/include/nodes/$f"; done
: > "$OUT/tags.ndjson"
for page in $(seq 1 "$TAG_PAGES"); do
  tmp="$OUT/tags-page-$page.json"
  if ! curl -L --fail --max-time 45 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$API_ROOT/tags?per_page=100&page=$page" -o "$tmp"; then break; fi
  python3 - "$tmp" "$OUT/tags.ndjson" <<'PY'
import json,pathlib,sys
arr=json.loads(pathlib.Path(sys.argv[1]).read_text())
with pathlib.Path(sys.argv[2]).open('a',encoding='utf-8') as f:
  for x in arr:
    f.write(json.dumps({'name':x.get('name'),'commit_sha':(x.get('commit') or {}).get('sha')},ensure_ascii=False)+'\n')
print(len(arr))
PY
  [ "$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$tmp")" -lt 100 ] && break
done
python3 - "$OUT" "$REF" <<'PY'
import hashlib,json,pathlib,sys,datetime,os
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for p in sorted([out/'tags.ndjson'] + list((out/'src').rglob('*'))):
    if p.is_file():
      b=p.read_bytes(); files.append({'path':str(p.relative_to(out)),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'PostgreSQL SQL grammar and plan-node metadata','retrieved_at':datetime.date.today().isoformat(),'ref':ref,'tag_pages_requested':int(os.environ.get('POSTGRES_TAG_PAGES','20')),'files':files,'policy':'SQL token/production/node-tag/struct identifiers only; source bodies/query logs/customer schemas/execution excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
