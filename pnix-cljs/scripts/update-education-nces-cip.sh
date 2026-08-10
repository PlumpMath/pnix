#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NCES_CIP_SRC:-$ROOT/ingest/education/nces-cip}"
YEAR_ID="${NCES_CIP_YEAR_ID:-56}"
URL="${NCES_CIP_URL:-https://nces.ed.gov/ipeds/cipcode/browse.aspx?y=${YEAR_ID}}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$YEAR_ID" "$URL" <<'PY'
import datetime as dt, hashlib, json, urllib.request, sys
from pathlib import Path

dest=Path(sys.argv[1]); year_id=sys.argv[2]; url=sys.argv[3]
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)
req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/nces-cip'})
with urllib.request.urlopen(req, timeout=30) as r:
    data=r.read(); headers=dict(r.headers); final_url=r.geturl()
(raw/'browse.html').write_bytes(data)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'NCES CIP browse tree','license':'US Government public domain / NCES public information','year_id':year_id,'url':url,'final_url':final_url,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),'content_type':headers.get('Content-Type',''),'scope':'CIP code hierarchy metadata only; detail prose/program rows/student data/graph wiring excluded'}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f'updated NCES CIP browse tree: year_id={year_id} bytes={len(data)} dest={dest}')
PY
