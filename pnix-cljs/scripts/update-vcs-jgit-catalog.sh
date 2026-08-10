#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${JGIT_DEST:-$ROOT/ingest/vcs/jgit}"
REPO="${JGIT_REPO:-eclipse-jgit/jgit}"
TAG_LIMIT="${JGIT_TAG_LIMIT:-100}"
USER_AGENT="${JGIT_USER_AGENT:-pnix-ingest/0.1 (JGit metadata catalog)}"
mkdir -p "$DEST/raw"
echo "JGit tags 수집: repo=$REPO limit=$TAG_LIMIT" >&2
curl -fsSL -A "$USER_AGENT" "https://api.github.com/repos/$REPO/tags?per_page=$TAG_LIMIT" -o "$DEST/raw/tags.json"
latest_sha="$(python3 - <<'PY' "$DEST/raw/tags.json"
import json,sys
j=json.load(open(sys.argv[1])); print(j[0]['commit']['sha'] if j else '')
PY
)"
if [ -n "$latest_sha" ]; then
  echo "JGit latest tree 수집: $latest_sha" >&2
  curl -fsSL -A "$USER_AGENT" "https://api.github.com/repos/$REPO/git/trees/$latest_sha?recursive=1" -o "$DEST/raw/latest-tree.json"
fi
python3 - <<'PY' "$DEST" "$REPO" "$TAG_LIMIT" "$latest_sha"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); repo=sys.argv[2]; tag_limit=int(sys.argv[3]); latest=sys.argv[4]
files=[]
for p in sorted((root/'raw').glob('*.json')):
 b=p.read_bytes(); files.append({'path':str(p.relative_to(root)), 'sha256':hashlib.sha256(b).hexdigest(), 'bytes':len(b)})
receipt={'schema':'vcs.jgit.catalog.source_receipt.v1','repo':repo,'tag_limit':tag_limit,'latest_commit_sha':latest,'retrieved_at_unix':int(time.time()),'license':'Eclipse Distribution License 1.0 / BSD-3-Clause','files':files,'excluded':['java source bodies','binary jars','patch/diff bodies','operation execution']}
(root/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
PY
