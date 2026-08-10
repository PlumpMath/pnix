#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NATURAL_EARTH_DEST:-$ROOT/ingest/place/natural-earth}"
REPO="${NATURAL_EARTH_REPO:-nvkelso/natural-earth-vector}"
TAG_LIMIT="${NATURAL_EARTH_TAG_LIMIT:-100}"
UA="${NATURAL_EARTH_USER_AGENT:-pnix-ingest/0.1 (Natural Earth catalog metadata)}"
mkdir -p "$DEST/raw"
curl -fsSL -A "$UA" "https://api.github.com/repos/$REPO/tags?per_page=$TAG_LIMIT" -o "$DEST/raw/tags.json"
sha="$(python3 - <<'PY' "$DEST/raw/tags.json"
import json,sys
j=json.load(open(sys.argv[1])); print(j[0]['commit']['sha'] if j else 'master')
PY
)"
curl -fsSL -A "$UA" "https://api.github.com/repos/$REPO/git/trees/$sha?recursive=1" -o "$DEST/raw/tree.json"
python3 - <<'PY' "$DEST" "$REPO" "$sha"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); repo=sys.argv[2]; sha=sys.argv[3]
files=[]
for p in sorted((root/'raw').glob('*.json')):
 b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'place.natural_earth_catalog.source_receipt.v1','repo':repo,'tree_sha':sha,'retrieved_at_unix':int(time.time()),'license':'Public Domain','files':files,'excluded':['geometry payloads','coordinates','raster payloads','rendering/geocoding decisions']},ensure_ascii=False,indent=2)+'\n')
PY
