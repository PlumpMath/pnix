#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${DULWICH_DEST:-$ROOT/ingest/vcs/dulwich}"
REPO="${DULWICH_REPO:-jelmer/dulwich}"
TAG_LIMIT="${DULWICH_TAG_LIMIT:-100}"
USER_AGENT="${DULWICH_USER_AGENT:-pnix-ingest/0.1 (Dulwich release metadata catalog)}"
mkdir -p "$DEST/raw"
echo "Dulwich PyPI metadata 수집" >&2
curl -fsSL -A "$USER_AGENT" "https://pypi.org/pypi/dulwich/json" -o "$DEST/raw/pypi.json"
echo "Dulwich GitHub tags 수집: $REPO limit=$TAG_LIMIT" >&2
curl -fsSL -A "$USER_AGENT" "https://api.github.com/repos/$REPO/tags?per_page=$TAG_LIMIT" -o "$DEST/raw/tags.json"
python3 - <<'PY' "$DEST" "$REPO" "$TAG_LIMIT"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); repo=sys.argv[2]; tag_limit=int(sys.argv[3])
files=[]
for p in sorted((root/'raw').glob('*.json')):
 b=p.read_bytes(); files.append({'path':str(p.relative_to(root)), 'sha256':hashlib.sha256(b).hexdigest(), 'bytes':len(b)})
receipt={'schema':'vcs.dulwich.catalog.source_receipt.v1','repo':repo,'tag_limit':tag_limit,'retrieved_at_unix':int(time.time()),'license':'Apache-2.0 OR GPL-2.0-or-later; Apache-2.0 path selected','files':files,'excluded':['wheel/sdist bodies','source code bodies','repository contents','Git operation execution']}
(root/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
PY
