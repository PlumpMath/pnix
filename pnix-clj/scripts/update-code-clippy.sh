#!/usr/bin/env bash
# Clippy lint metadata source updater.
# Downloads rust-lang/rust-clippy source snapshot and records provenance. No graph/mirror wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/clippy"
REF="${CLIPPY_REF:-master}"
mkdir -p "$DEST"
if [[ "$REF" == "master" || "$REF" == "latest" ]]; then
  API="https://api.github.com/repos/rust-lang/rust-clippy/commits/master"
else
  API="https://api.github.com/repos/rust-lang/rust-clippy/commits/$REF"
fi
META="$DEST/commit.json"
curl -fsSL "$API" -o "$META"
SHA="$(python3 - "$META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['sha'])
PY
)"
DATE="$(python3 - "$META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['commit']['committer']['date'])
PY
)"
URL="https://github.com/rust-lang/rust-clippy/archive/$SHA.tar.gz"
ARCHIVE="$DEST/clippy-$SHA.tar.gz"
TMP="$ARCHIVE.tmp"
curl -fL "$URL" -o "$TMP"
ARCHIVE_SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
mv "$TMP" "$ARCHIVE"
rm -rf "$DEST/src"
mkdir -p "$DEST/src"
tar -xzf "$ARCHIVE" -C "$DEST/src" --strip-components=1
python3 - "$DEST/source-receipt.json" "$SHA" "$DATE" "$URL" "$ARCHIVE_SHA" "$ARCHIVE" <<'PY'
import json,sys,datetime,os
out,sha,date,url,archive_sha,archive=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'Clippy lint metadata',
  'repository':'rust-lang/rust-clippy',
  'version':sha,
  'commit_sha':sha,
  'commit_date':date,
  'url':url,
  'sha256':archive_sha,
  'size_bytes':os.path.getsize(archive),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'MIT OR Apache-2.0',
  'scope':'lint declaration/category/rename/deprecation metadata only; no implementation body/docs prose/user lint results/configs/autofix payloads'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated Clippy source: $SHA $ARCHIVE_SHA"
