#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/usgs-earthquake-products-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
SHAKEMAP_URL="${USGS_SHAKEMAP_URL:-https://earthquake.usgs.gov/data/shakemap/}"
PAGER_URL="${USGS_PAGER_URL:-https://earthquake.usgs.gov/data/pager/}"
curl -fsSL -A "$UA" "$SHAKEMAP_URL" -o "$OUT/shakemap.html"
curl -fsSL -A "$UA" "$PAGER_URL" -o "$OUT/pager.html"
python3 - "$OUT" "$SHAKEMAP_URL" "$PAGER_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'usgs-earthquake-products-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'USGS ShakeMap/PAGER catalog pages only; grids/loss estimates/alerts/response guidance/event payloads excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
