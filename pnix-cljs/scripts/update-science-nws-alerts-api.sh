#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NWS_ALERTS_API_SRC:-$ROOT/ingest/science/nws-alerts-api}"
OPENAPI_URL="${NWS_OPENAPI_URL:-https://api.weather.gov/openapi.json}"
TYPES_URL="${NWS_ALERT_TYPES_URL:-https://api.weather.gov/alerts/types}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$OPENAPI_URL" "$TYPES_URL" <<'PY'
import datetime as dt, hashlib, json, urllib.request, sys
from pathlib import Path

dest=Path(sys.argv[1]); openapi_url=sys.argv[2]; types_url=sys.argv[3]
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)

def fetch(url):
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/nws-alerts-api'})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read(), dict(r.headers), r.geturl()
files=[]
for name,url in [('openapi.json',openapi_url),('alert-types.json',types_url)]:
    data, headers, final_url = fetch(url)
    (raw/name).write_bytes(data)
    files.append({'name':name,'url':url,'final_url':final_url,'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),'content_type':headers.get('Content-Type','')})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'National Weather Service API OpenAPI / alert type metadata','license':'US Government public domain / public API metadata','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'scope':'OpenAPI structure and alert type taxonomy only; live alerts/CAP/forecast/zone geometry/emergency guidance/graph wiring excluded','files':files}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f'updated NWS alerts API metadata: files={len(files)} dest={dest}')
PY
