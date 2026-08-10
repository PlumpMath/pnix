#!/usr/bin/env bash
# Conventional Commits spec source updater.
# Fetches the current master snapshot. There are no official GitHub release assets.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/conventional-commits"
REF="${CONVENTIONAL_COMMITS_REF:-master}"
REPO="conventional-commits/conventionalcommits.org"
mkdir -p "$DEST"
COMMIT_JSON="$DEST/commit.json"
curl -fsSL "https://api.github.com/repos/$REPO/commits/$REF" -o "$COMMIT_JSON"
SHA="$(python3 - "$COMMIT_JSON" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['sha'])
PY
)"
DATE="$(python3 - "$COMMIT_JSON" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['commit']['committer']['date'])
PY
)"
URL="https://api.github.com/repos/$REPO/tarball/$SHA"
ARCHIVE="$DEST/conventionalcommits-$SHA.tar.gz"
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
  'source':'Conventional Commits specification',
  'repository':'conventional-commits/conventionalcommits.org',
  'ref':'master',
  'commit_sha':sha,
  'commit_date':date,
  'spec_version':'1.0.0',
  'url':url,
  'sha256':archive_sha,
  'size_bytes':os.path.getsize(archive),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'MIT',
  'scope':'commit-message syntax/structural metadata only; no repository commit logs or user messages'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated Conventional Commits source: $SHA $ARCHIVE_SHA"
