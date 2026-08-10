#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/climate/nasa-power-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (API catalog metadata only; contact: local)"
DOCS_URL="${NASA_POWER_DOCS_URL:-https://power.larc.nasa.gov/docs/services/api/}"
PAGES_URL="${NASA_POWER_PAGES_URL:-https://power.larc.nasa.gov/api/pages/}"
TUTORIAL_URL="${NASA_POWER_TUTORIAL_URL:-https://power.larc.nasa.gov/docs/tutorials/service-data-request/api/}"
curl -fsSL -A "$UA" "$DOCS_URL" -o "$OUT/api-docs.html"
curl -fsSL -A "$UA" "$PAGES_URL" -o "$OUT/api-pages.html"
curl -fsSL -A "$UA" "$TUTORIAL_URL" -o "$OUT/tutorial-api.html"
python3 - "$OUT" "$DOCS_URL" "$PAGES_URL" "$TUTORIAL_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'nasa-power-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'NASA POWER documentation/catalog metadata only; API data payloads/FIRMS/life-safety guidance excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
