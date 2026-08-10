#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/rust-reference"
mkdir -p "$OUT/src"
REF="${RUST_REFERENCE_REF:-master}"
TAG_PAGES="${RUST_REFERENCE_TAG_PAGES:-20}"
API_ROOT="${RUST_REFERENCE_API_ROOT:-https://api.github.com/repos/rust-lang/reference}"
RUST_RELEASE_TAGS_API="${RUST_RELEASE_TAGS_API:-https://api.github.com/repos/rust-lang/rust/tags}"
RAW_ROOT="https://raw.githubusercontent.com/rust-lang/reference/$REF"
curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$API_ROOT/git/trees/$REF?recursive=1" -o "$OUT/tree.json"
: > "$OUT/tags.ndjson"
for page in $(seq 1 "$TAG_PAGES"); do
  tmp="$OUT/tags-page-$page.json"
  if ! curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$API_ROOT/tags?per_page=100&page=$page" -o "$tmp"; then break; fi
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
: > "$OUT/rust-release-tags.ndjson"
for page in $(seq 1 "$TAG_PAGES"); do
  tmp="$OUT/rust-release-tags-page-$page.json"
  if ! curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$RUST_RELEASE_TAGS_API?per_page=100&page=$page" -o "$tmp"; then break; fi
  python3 - "$tmp" "$OUT/rust-release-tags.ndjson" <<'PY'
import json,pathlib,sys
arr=json.loads(pathlib.Path(sys.argv[1]).read_text())
with pathlib.Path(sys.argv[2]).open('a',encoding='utf-8') as f:
  for x in arr:
    f.write(json.dumps({'name':x.get('name'),'commit_sha':(x.get('commit') or {}).get('sha')},ensure_ascii=False)+'\n')
print(len(arr))
PY
  [ "$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$tmp")" -lt 100 ] && break
done
python3 - "$OUT/tree.json" "$OUT/paths.txt" <<'PY'
import json,pathlib,sys
tree=json.loads(pathlib.Path(sys.argv[1]).read_text()).get('tree',[])
paths=[x['path'] for x in tree if x.get('type')=='blob' and x.get('path','').startswith('src/') and x.get('path','').endswith('.md')]
pathlib.Path(sys.argv[2]).write_text('\n'.join(paths[:120])+'\n',encoding='utf-8')
print(len(paths[:120]))
PY
while IFS= read -r path; do
  [ -z "$path" ] && continue
  dest="$OUT/$path"
  mkdir -p "$(dirname "$dest")"
  curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$RAW_ROOT/$path" -o "$dest"
done < "$OUT/paths.txt"
python3 - "$OUT" "$REF" <<'PY'
import hashlib,json,pathlib,sys,datetime,os
out=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for p in sorted([out/'tree.json', out/'tags.ndjson', out/'rust-release-tags.ndjson', out/'paths.txt'] + list((out/'src').rglob('*.md'))):
    if p.exists():
      b=p.read_bytes(); files.append({'path':str(p.relative_to(out)),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'Rust Reference structural metadata','retrieved_at':datetime.date.today().isoformat(),'ref':ref,'tag_pages_requested':int(os.environ.get('RUST_REFERENCE_TAG_PAGES','20')),'files':files,'policy':'tag catalog + path/heading/grammar-codeblock metadata only; prose/examples/source execution excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
