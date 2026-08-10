#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/kigam-geodata-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (KIGAM geodata catalog metadata only; no GIS payloads)"
BASE="${KIGAM_GEODATA_BASE_URL:-https://data.kigam.re.kr}"
declare -A urls=(
  [home]="$BASE/"
  [openapi_guide]="$BASE/guide/openapi"
  [geologic_250k]="${KIGAM_GEODATA_250K_URL:-$BASE/data/ac9b5c66-1768-447a-b28d-0eb0111ea401}"
  [geologic_1000k]="${KIGAM_GEODATA_1000K_URL:-$BASE/data/f4e8c444-5039-4331-9cd0-f1474ffdaed1}"
  [search_250k]="$BASE/search?q=%EC%88%98%EC%B9%98%EC%A7%80%EC%A7%88%EB%8F%84_25%EB%A7%8C%EC%B6%95%EC%B2%99"
  [search_1000k]="$BASE/search?q=%EC%88%98%EC%B9%98%EC%A7%80%EC%A7%88%EB%8F%84_100%EB%A7%8C%EC%B6%95%EC%B2%99"
)
for k in "${!urls[@]}"; do
  curl -fsSL --retry 2 --max-time 30 -A "$UA" "${urls[$k]}" -o "$OUT/$k.html"
done
python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1])
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'kigam-geodata-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'files':files,'policy':'KIGAM Geo Big Data catalog metadata only; GIS/SHP/map payloads, feature geometries, report bodies, credentials and geohazard advice excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
