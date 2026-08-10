#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/transport/bts-ntad-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
GEODATA_URL="${BTS_GEODATA_URL:-https://data-usdot.opendata.arcgis.com/}"
DCAT_URL="${BTS_NTAD_DCAT_URL:-https://data-usdot.opendata.arcgis.com/api/feed/dcat-us/1.1.json}"
curl -fsSL -A "$UA" "$GEODATA_URL" -o "$OUT/bts-geodata.html"
curl -fsSL -A "$UA" "$DCAT_URL" -o "$OUT/dcat-us.json"
python3 - "$OUT" "$GEODATA_URL" "$DCAT_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'bts-ntad-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'BTS Geospatial Hub DCAT/catalog metadata only; geospatial payloads/geometries/coordinates/prose bodies excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
