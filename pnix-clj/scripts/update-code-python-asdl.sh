#!/usr/bin/env bash
# CPython Parser/Python.asdl 원본 업데이트.
# 언어별 버전은 전부 원칙: --all-tags 로 전체 tag catalog를 저장하고, --tag 로 특정 버전을 받는다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUTDIR="$ROOT/ingest/code/python"
TAG=""
ALL_TAGS=0
while [ $# -gt 0 ]; do
  case "$1" in
    --tag) TAG="$2"; shift 2;;
    --all-tags) ALL_TAGS=1; shift;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
mkdir -p "$OUTDIR"
if [ "$ALL_TAGS" = 1 ]; then
  python3 - "$OUTDIR/tags.json" <<'PY'
import json, urllib.request, sys
out = sys.argv[1]
all_tags = []
page = 1
while True:
    url = f'https://api.github.com/repos/python/cpython/tags?per_page=100&page={page}'
    with urllib.request.urlopen(url, timeout=30) as r:
        data = json.load(r)
    if not data:
        break
    all_tags.extend(data)
    page += 1
open(out, 'w', encoding='utf-8').write(json.dumps(all_tags, ensure_ascii=False, indent=2) + '\n')
print(f'wrote {out}: {len(all_tags)} tags')
PY
fi
if [ -z "$TAG" ]; then
  TAG=$(python3 - <<'PY'
import json, re, urllib.request
with urllib.request.urlopen('https://api.github.com/repos/python/cpython/tags?per_page=100', timeout=30) as r:
    data=json.load(r)
for row in data:
    n=row['name']
    if re.match(r'^v\d+\.\d+\.\d+$', n):
        print(n)
        break
PY
)
fi
BASE="https://raw.githubusercontent.com/python/cpython/${TAG}"
curl -fsSL "$BASE/Parser/Python.asdl" -o "$OUTDIR/Python.asdl"
curl -fsSL "$BASE/LICENSE" -o "$OUTDIR/LICENSE"
python3 - "$OUTDIR/manifest.json" "$TAG" "$OUTDIR/Python.asdl" "$OUTDIR/LICENSE" <<'PY'
import hashlib, json, sys
manifest, tag, asdl, license_file = sys.argv[1:]
def sha(path):
    return hashlib.sha256(open(path,'rb').read()).hexdigest()
obj = {
  'schema': 'pnix.ingest.source_manifest.v1',
  'source_id': 'python-asdl',
  'project': 'python/cpython',
  'tag': tag,
  'source_path': 'Parser/Python.asdl',
  'license_path': 'LICENSE',
  'source_sha256': sha(asdl),
  'license_sha256': sha(license_file),
}
open(manifest, 'w', encoding='utf-8').write(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(obj, ensure_ascii=False))
PY
