#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/usgs-ngmdb-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
HOME_URL="${USGS_NGMDB_HOME_URL:-https://ngmdb.usgs.gov/ngmdb/ngmdb_home.html}"
MAPVIEW_URL="${USGS_NGMDB_MAPVIEW_URL:-https://ngmdb.usgs.gov/mapview/}"
PRODUCT_URL="${USGS_NGMDB_PRODUCT_URL:-https://ngmdb.usgs.gov/Prodesc/proddesc_86688.htm}"
curl -fsSL -A "$UA" "$HOME_URL" -o "$OUT/ngmdb-home.html"
curl -fsSL -A "$UA" "$MAPVIEW_URL" -o "$OUT/mapview.html"
curl -fsSL -A "$UA" "$PRODUCT_URL" -o "$OUT/product-sample.html"
python3 - "$OUT" "$HOME_URL" "$MAPVIEW_URL" "$PRODUCT_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'usgs-ngmdb-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'NGMDB index/catalog page metadata only; map sheets/GIS/PDF/publisher payloads and geohazard/stability judgments excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
