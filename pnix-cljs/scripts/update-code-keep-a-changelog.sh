#!/usr/bin/env bash
# Keep a Changelog source updater.
# Fetches the latest olivierlacan/keep-a-changelog release tarball.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/keep-a-changelog"
TAG="${KEEP_A_CHANGELOG_TAG:-latest}"
mkdir -p "$DEST"
API="https://api.github.com/repos/olivierlacan/keep-a-changelog/releases"
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
ARCHIVE="$DEST/keep-a-changelog-$TAG.tar.gz"
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
  'source':'Keep a Changelog',
  'repository':'olivierlacan/keep-a-changelog',
  'version':tag,
  'published_at':date,
  'url':url,
  'sha256':sha,
  'size_bytes':os.path.getsize(archive),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'MIT',
  'scope':'changelog section taxonomy/structure metadata only; project-specific entries and prose body excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated Keep a Changelog source: $TAG $SHA"
