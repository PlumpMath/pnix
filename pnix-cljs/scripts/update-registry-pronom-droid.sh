#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${PRONOM_DROID_SRC:-$ROOT/ingest/registry/pronom-droid}"
VERSION="${PRONOM_DROID_VERSION:-119}"
URL="${PRONOM_DROID_URL:-https://cdn.nationalarchives.gov.uk/documents/DROID_SignatureFile_V${VERSION}.xml}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$VERSION" "$URL" <<'PY'
import datetime as dt, hashlib, json, urllib.request, sys
from pathlib import Path

dest=Path(sys.argv[1]); version=sys.argv[2]; url=sys.argv[3]
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)
req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/pronom-droid'})
with urllib.request.urlopen(req, timeout=60) as r:
    data=r.read(); headers=dict(r.headers); final_url=r.geturl()
(raw/'DROID_SignatureFile.xml').write_bytes(data)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'PRONOM DROID Signature File','license':'OGL-UK-3.0','version':version,'url':url,'final_url':final_url,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),'content_type':headers.get('Content-Type',''),'scope':'bounded file format identifier/signature metadata only; no sample files/document payloads/parser execution/graph wiring'}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f'updated PRONOM DROID signature file: version={version} bytes={len(data)} dest={dest}')
PY
