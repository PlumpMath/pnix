#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/agriculture/usda-plants-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (app/API catalog metadata only; contact: local)"
HOME_URL="${USDA_PLANTS_HOME_URL:-https://plants.sc.egov.usda.gov/home}"
CONFIG_URL="${USDA_PLANTS_CONFIG_URL:-https://plants.sc.egov.usda.gov/assets/config.json}"
MAIN_JS_URL="${USDA_PLANTS_MAIN_JS_URL:-https://plants.sc.egov.usda.gov/main.js}"
curl -fsSL -A "$UA" "$HOME_URL" -o "$OUT/home.html"
curl -fsSL -A "$UA" "$CONFIG_URL" -o "$OUT/config.json"
curl -fsSL -A "$UA" "$MAIN_JS_URL" -o "$OUT/main.js"
python3 - "$OUT" "$HOME_URL" "$CONFIG_URL" "$MAIN_JS_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'usda-plants-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'application config and endpoint token metadata only; plant payloads/maps/documents/guidance excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
