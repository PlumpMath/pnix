#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/climate/nasa-firms-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (FIRMS API catalog metadata only; no data payload calls)"
urls=(
  "${NASA_FIRMS_API_URL:-https://firms.modaps.eosdis.nasa.gov/api/}"
  "${NASA_FIRMS_AREA_URL:-https://firms.modaps.eosdis.nasa.gov/api/area/}"
  "${NASA_FIRMS_DATA_AVAILABILITY_URL:-https://firms.modaps.eosdis.nasa.gov/api/data_availability/}"
  "${NASA_FIRMS_KML_FIRE_FOOTPRINTS_URL:-https://firms.modaps.eosdis.nasa.gov/api/kml_fire_footprints/}"
  "${NASA_FIRMS_MISSING_DATA_URL:-https://firms.modaps.eosdis.nasa.gov/api/missing_data/}"
  "${NASA_FIRMS_MAP_KEY_URL:-https://firms.modaps.eosdis.nasa.gov/api/map_key/}"
  "${NASA_FIRMS_EARTHDATA_URL:-https://www.earthdata.nasa.gov/data/tools/firms}"
)
names=(api area data_availability kml_fire_footprints missing_data map_key earthdata_tool)
for i in "${!urls[@]}"; do
  curl -fsSL --retry 2 --max-time 30 -A "$UA" "${urls[$i]}" -o "$OUT/${names[$i]}.html"
done
python3 - "$OUT" "${urls[@]}" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={
  'schema':'pnix.source_manifest.v1',
  'source_id':'nasa-firms-api-catalog',
  'retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'source_urls':urls,
  'files':files,
  'policy':'NASA FIRMS documentation/catalog metadata only; active fire payloads, MAP_KEY credentials, map tiles, CSV/GeoJSON/KML payloads, and life-safety guidance excluded'
}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
