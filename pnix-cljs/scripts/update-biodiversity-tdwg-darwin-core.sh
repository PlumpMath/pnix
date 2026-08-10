#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${TDWG_DWC_SRC:-$ROOT/ingest/biodiversity/tdwg-darwin-core}"
REF="${TDWG_DWC_REF:-master}"
CSV_URL="${TDWG_DWC_CSV_URL:-https://raw.githubusercontent.com/tdwg/dwc/$REF/vocabulary/term_versions.csv}"
LICENSE_URL="${TDWG_DWC_LICENSE_URL:-https://raw.githubusercontent.com/tdwg/dwc/$REF/LICENSE}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" "$CSV_URL" "$LICENSE_URL" <<'PY'
import datetime as dt, hashlib, json, urllib.request, sys
from pathlib import Path

dest=Path(sys.argv[1]); ref=sys.argv[2]; csv_url=sys.argv[3]; license_url=sys.argv[4]
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)

def fetch(url):
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/tdwg-darwin-core'})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read(), dict(r.headers), r.geturl()
files=[]
for name,url in [('term_versions.csv',csv_url),('LICENSE',license_url)]:
    data, headers, final_url = fetch(url)
    (raw/name).write_bytes(data)
    files.append({'name':name,'url':url,'final_url':final_url,'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),'content_type':headers.get('Content-Type','')})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'TDWG Darwin Core vocabulary term metadata','license':'CC-BY-4.0','ref':ref,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'scope':'term_versions.csv structural metadata only; definition/comment/example prose, occurrence rows, sensitive coordinates, media, personal data, graph wiring excluded','files':files}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f'updated TDWG Darwin Core: files={len(files)} ref={ref} dest={dest}')
PY
