#!/usr/bin/env bash
# TypeScript SyntaxKind 원본 업데이트.
# 기본은 latest release 1개를 받지만, --all-tags 를 주면 GitHub tag catalog를 내려받아 버전별 ingest 작업표를 만든다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUTDIR="$ROOT/ingest/code/typescript"
TAG=""
ALL_TAGS=0
while [ $# -gt 0 ]; do
  case "$1" in
    --tag) TAG="$2"; shift 2;;
    --latest) TAG=""; shift;;
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
    url = f'https://api.github.com/repos/microsoft/TypeScript/tags?per_page=100&page={page}'
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
import json, urllib.request
with urllib.request.urlopen('https://api.github.com/repos/microsoft/TypeScript/releases/latest', timeout=30) as r:
    print(json.load(r)['tag_name'])
PY
)
fi
BASE="https://raw.githubusercontent.com/microsoft/TypeScript/${TAG}"
curl -fsSL "$BASE/src/compiler/types.ts" -o "$OUTDIR/types.ts"
curl -fsSL "$BASE/LICENSE.txt" -o "$OUTDIR/LICENSE.txt"
python3 - "$OUTDIR/manifest.json" "$TAG" "$OUTDIR/types.ts" "$OUTDIR/LICENSE.txt" <<'PY'
import hashlib, json, sys
manifest, tag, types, license_file = sys.argv[1:]
def sha(path):
    return hashlib.sha256(open(path,'rb').read()).hexdigest()
obj = {
  'schema': 'pnix.ingest.source_manifest.v1',
  'source_id': 'typescript-syntaxkind',
  'project': 'microsoft/TypeScript',
  'tag': tag,
  'source_path': 'src/compiler/types.ts',
  'license_path': 'LICENSE.txt',
  'source_sha256': sha(types),
  'license_sha256': sha(license_file),
}
open(manifest, 'w', encoding='utf-8').write(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(obj, ensure_ascii=False))
PY
