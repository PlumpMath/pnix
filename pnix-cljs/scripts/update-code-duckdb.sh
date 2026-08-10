#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/code/duckdb"
API="${DUCKDB_RELEASE_API:-https://api.github.com/repos/duckdb/duckdb/releases/latest}"
mkdir -p "$OUT_DIR"
curl -fsSL "$API" -o "$OUT_DIR/release.json"
python3 - "$OUT_DIR/release.json" > "$OUT_DIR/asset_url" <<'PY'
import json, platform, sys
j=json.load(open(sys.argv[1]))
machine=platform.machine().lower()
want='duckdb_cli-osx-arm64.zip' if machine in ('arm64','aarch64') else 'duckdb_cli-osx-amd64.zip'
for a in j['assets']:
    if a['name']==want:
        print(a['browser_download_url'])
        raise SystemExit(0)
raise SystemExit(f'missing {want}')
PY
TAG=$(python3 - "$OUT_DIR/release.json" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tag_name'])
PY
)
URL="$(cat "$OUT_DIR/asset_url")"
ZIP="$OUT_DIR/duckdb-cli-$TAG.zip"
TMP="$ZIP.tmp.$$"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$ZIP"
shasum -a 256 "$ZIP" > "$ZIP.sha256"
rm -rf "$OUT_DIR/bin" && mkdir -p "$OUT_DIR/bin"
unzip -q "$ZIP" -d "$OUT_DIR/bin"
chmod +x "$OUT_DIR/bin/duckdb"
{
  echo "source_url=$URL"
  echo "tag=$TAG"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$ZIP.sha256")"
} > "$ZIP.meta"
echo "wrote: $ZIP"
cat "$ZIP.sha256"
