#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${GNIS_DEST:-$ROOT/ingest/place/gnis}"
UA="${GNIS_USER_AGENT:-pnix-ingest/0.1 (GNIS feature class metadata)}"
mkdir -p "$DEST/raw"
curl -fsSL -A "$UA" "https://www.usgs.gov/us-board-on-geographic-names/gnis-domestic-names-feature-classes" -o "$DEST/raw/feature-classes.html"
curl -fsSL -A "$UA" "https://www.usgs.gov/us-board-on-geographic-names/download-gnis-data" -o "$DEST/raw/download-gnis-data.html"
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*.html')):
 b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'place.gnis_feature_classes.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / public information','files':files,'excluded':['GNIS name rows','coordinates','variant names','definitions prose']},ensure_ascii=False,indent=2)+'\n')
PY
