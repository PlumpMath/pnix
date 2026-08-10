#!/usr/bin/env bash
# Ruff rule metadata source updater.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/ruff"
TAG="${RUFF_TAG:-latest}"
mkdir -p "$DEST"
API="https://api.github.com/repos/astral-sh/ruff/releases"
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
ARCHIVE="$DEST/ruff-$TAG.tar.gz"
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
  'source':'Ruff rule metadata',
  'repository':'astral-sh/ruff',
  'version':tag,
  'published_at':date,
  'url':url,
  'sha256':sha,
  'size_bytes':os.path.getsize(archive),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'MIT',
  'scope':'rule code/linter/redirect metadata only; no rule implementation/docs prose/user lint results/configs/autofix payloads'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated Ruff source: $TAG $SHA"
