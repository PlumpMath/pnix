#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/water/epa-echo-sdwa-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (API catalog metadata only; contact: local)"
WEB_URL="${EPA_ECHO_WEB_SERVICES_URL:-https://echo.epa.gov/tools/web-services}"
SDWA_URL="${EPA_ECHO_SDWA_DOCS_URL:-https://echo.epa.gov/tools/web-services/facility-search-drinking-water}"
curl -fsSL -A "$UA" "$WEB_URL" -o "$OUT/web-services.html"
# SDWA-specific URL currently returns documentation page or 404 shell depending on deployment; keep status metadata only.
curl -fsSL -A "$UA" "$SDWA_URL" -o "$OUT/sdwa-docs.html" || true
python3 - "$OUT" "$WEB_URL" "$SDWA_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'epa-echo-sdwa-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'EPA ECHO web-services catalog metadata only; SDWA/SDWIS payload values and safety/legal judgments excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
