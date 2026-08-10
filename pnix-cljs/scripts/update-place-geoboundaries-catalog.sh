#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${GEOBOUNDARIES_DEST:-$ROOT/ingest/place/geoboundaries}"
UA="${GEOBOUNDARIES_USER_AGENT:-pnix-ingest/0.1 (geoBoundaries metadata catalog)}"
mkdir -p "$DEST/raw"
base="https://www.geoboundaries.org/api/current/gbOpen"
curl -fsSL -A "$UA" "$base/ALL/ADM0/" -o "$DEST/raw/gbopen-all-adm0.json"
for iso in ${GEOBOUNDARIES_ADM1_ISOS:-USA KOR FRA BRA IND NGA}; do
  curl -fsSL -A "$UA" "$base/$iso/ADM1/" -o "$DEST/raw/gbopen-${iso}-adm1.json" || true
done
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*.json')):
 b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'place.geoboundaries_catalog.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'CC-BY-4.0 for gbOpen; per-record license retained','files':files,'excluded':['geometry files','coordinate payloads','gbHumanitarian','gbAuthoritative']},ensure_ascii=False,indent=2)+'\n')
PY
