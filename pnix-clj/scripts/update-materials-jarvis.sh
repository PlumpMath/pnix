#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/materials/jarvis"
ARTICLE_API="${JARVIS_ARTICLE_API:-https://api.figshare.com/v2/articles/6815699}"
FILE_NAME="${JARVIS_FILE_NAME:-jdft_3d-9-24-2025.json.zip}"
mkdir -p "$OUT_DIR"
curl -fsSL "$ARTICLE_API" -o "$OUT_DIR/article.json"
python3 - "$OUT_DIR/article.json" "$FILE_NAME" > "$OUT_DIR/download_url" <<'PY'
import json, sys
article=json.load(open(sys.argv[1]))
want=sys.argv[2]
for f in article['files']:
    if f['name']==want:
        print(f['download_url'])
        raise SystemExit(0)
raise SystemExit(f'missing file {want}')
PY
URL="$(cat "$OUT_DIR/download_url")"
ZIP="$OUT_DIR/$FILE_NAME"
TMP="$ZIP.tmp.$$"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$ZIP"
shasum -a 256 "$ZIP" > "$ZIP.sha256"
python3 - "$OUT_DIR/article.json" "$FILE_NAME" > "$ZIP.meta" <<'PY'
import json, sys
article=json.load(open(sys.argv[1])); want=sys.argv[2]
print('article_id='+str(article.get('id')))
print('article_version='+str(article.get('version')))
print('doi='+str(article.get('doi')))
print('license='+article.get('license',{}).get('name',''))
for f in article['files']:
    if f['name']==want:
        print('file_name='+f['name'])
        print('file_id='+str(f['id']))
        print('download_url='+f['download_url'])
        print('computed_md5='+str(f.get('computed_md5','')))
PY
rm -rf "$OUT_DIR/unpacked"
mkdir -p "$OUT_DIR/unpacked"
unzip -o "$ZIP" -d "$OUT_DIR/unpacked" >/dev/null
echo "wrote: $ZIP"
cat "$ZIP.sha256"
