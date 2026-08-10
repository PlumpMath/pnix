#!/usr/bin/env bash
# ESLint core rule metadata source updater.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/eslint"
TAG="${ESLINT_TAG:-latest}"
mkdir -p "$DEST"
API="https://api.github.com/repos/eslint/eslint/releases"
if [[ "$TAG" == "latest" ]]; then
  REL_URL="$API/latest"
else
  REL_URL="$API/tags/$TAG"
fi
REL="$DEST/release.json"
curl -fsSL "$REL_URL" -o "$REL"
TAG="$(python3 - "$REL" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tag_name'])
PY
)"
DATE="$(python3 - "$REL" <<'PY'
import json,sys
print(json.load(open(sys.argv[1])).get('published_at',''))
PY
)"
URL="$(python3 - "$REL" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tarball_url'])
PY
)"
ARCHIVE="$DEST/eslint-$TAG.tar.gz"
TMP="$ARCHIVE.tmp"
curl -fL "$URL" -o "$TMP"
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
mv "$TMP" "$ARCHIVE"
rm -rf "$DEST/src"
mkdir -p "$DEST/src"
tar -xzf "$ARCHIVE" -C "$DEST/src" --strip-components=1
python3 - "$DEST/source-receipt.json" "$TAG" "$DATE" "$URL" "$SHA" "$ARCHIVE" <<'PY'
import json,sys,datetime,os
out,tag,date,url,sha,archive=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'ESLint core rule metadata',
  'repository':'eslint/eslint',
  'version':tag,
  'published_at':date,
  'url':url,
  'sha256':sha,
  'size_bytes':os.path.getsize(archive),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'MIT',
  'scope':'core rule id/category/recommended/fixable/suggestion/deprecation metadata only; no user code/lint results/configs/autofix payloads'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated ESLint source: $TAG $SHA"
