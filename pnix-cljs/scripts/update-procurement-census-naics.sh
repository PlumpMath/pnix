#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CENSUS_NAICS_SRC:-$ROOT/ingest/procurement/census-naics}"
YEAR="${CENSUS_NAICS_YEAR:-2022}"
URL="${CENSUS_NAICS_URL:-https://www.census.gov/naics/${YEAR}NAICS/${YEAR}_NAICS_Structure.xlsx}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$YEAR" "$URL" <<'PY'
import datetime as dt, hashlib, json, urllib.request, sys
from pathlib import Path

dest=Path(sys.argv[1]); year=sys.argv[2]; url=sys.argv[3]
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)
req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/census-naics'})
with urllib.request.urlopen(req, timeout=30) as r:
    data=r.read(); headers=dict(r.headers); final_url=r.geturl()
(raw/'naics-structure.xlsx').write_bytes(data)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'U.S. Census NAICS Structure','license':'US-GOV-PD','year':year,'url':url,'final_url':final_url,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),'content_type':headers.get('Content-Type',''),'scope':'NAICS code/title hierarchy only; descriptions/examples/statistics/business records/graph wiring excluded'}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f'updated Census NAICS structure: year={year} bytes={len(data)} dest={dest}')
PY
