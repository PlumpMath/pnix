#!/usr/bin/env bash
# Cedar language metadata source updater.
# Downloads the latest cedar-policy/cedar source archive for grammar/schema extraction.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/cedar"
TAG="${CEDAR_TAG:-latest}"
mkdir -p "$DEST"
API="https://api.github.com/repos/cedar-policy/cedar/releases"
if [[ "$TAG" == "latest" ]]; then
  REL_URL="$API/latest"
else
  REL_URL="$API/tags/$TAG"
fi
META="$DEST/release.json"
curl -fsSL "$REL_URL" -o "$META"
TAG="$(python3 - "$META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tag_name'])
PY
)"
TARBALL_URL="$(python3 - "$META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tarball_url'])
PY
)"
ARCHIVE="$DEST/cedar-$TAG.tar.gz"
TMP="$ARCHIVE.tmp"
curl -fL "$TARBALL_URL" -o "$TMP"
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
mv "$TMP" "$ARCHIVE"
rm -rf "$DEST/src"
mkdir -p "$DEST/src"
tar -xzf "$ARCHIVE" -C "$DEST/src" --strip-components=1
python3 - "$DEST/source-receipt.json" "$TAG" "$TARBALL_URL" "$SHA" "$ARCHIVE" <<'PY'
import json,sys,datetime,os
out,tag,url,sha,archive=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'Cedar Policy Language',
  'repository':'cedar-policy/cedar',
  'version':tag,
  'url':url,
  'sha256':sha,
  'size_bytes':os.path.getsize(archive),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'Apache-2.0',
  'scope':'language grammar/schema metadata only; no actual policies or production authorization data'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated Cedar source: $TAG $SHA"
