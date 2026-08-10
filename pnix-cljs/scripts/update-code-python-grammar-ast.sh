#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/python-grammar-ast"
mkdir -p "$OUT"
REF="${PYTHON_GRAMMAR_REF:-main}"
TAG_PAGES="${PYTHON_TAG_PAGES:-50}"
BASE_RAW="https://raw.githubusercontent.com/python/cpython/$REF"
API_TAGS="${PYTHON_TAGS_API:-https://api.github.com/repos/python/cpython/tags}"
fetch() { curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$1" -o "$2"; }
fetch "$BASE_RAW/Grammar/python.gram" "$OUT/python.gram"
fetch "$BASE_RAW/Parser/Python.asdl" "$OUT/Python.asdl"
: > "$OUT/tags.ndjson"
for page in $(seq 1 "$TAG_PAGES"); do
  tmp="$OUT/tags-page-$page.json"
  if ! curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$API_TAGS?per_page=100&page=$page" -o "$tmp"; then
    break
  fi
  python3 - "$tmp" "$OUT/tags.ndjson" <<'PY'
import json, pathlib, sys
arr=json.loads(pathlib.Path(sys.argv[1]).read_text())
out=pathlib.Path(sys.argv[2])
with out.open('a',encoding='utf-8') as f:
    for x in arr:
        f.write(json.dumps({'name':x.get('name'), 'zipball_url':x.get('zipball_url'), 'tarball_url':x.get('tarball_url'), 'commit_sha':(x.get('commit') or {}).get('sha'), 'commit_url':(x.get('commit') or {}).get('url')},ensure_ascii=False)+'\n')
print(len(arr))
PY
  [ "$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$tmp")" -lt 100 ] && break
done
python3 - "$OUT" "$REF" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for p in sorted([out/'python.gram', out/'Python.asdl', out/'tags.ndjson']):
    b=p.read_bytes()
    files.append({'path':p.name,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'CPython grammar and AST schema metadata','retrieved_at':datetime.date.today().isoformat(),'ref':ref,'tag_pages_requested':int(__import__('os').environ.get('PYTHON_TAG_PAGES','12')),'files':files,'policy':'tag catalog + grammar/ASDL structural identifiers only; runtime source/prose/examples excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
