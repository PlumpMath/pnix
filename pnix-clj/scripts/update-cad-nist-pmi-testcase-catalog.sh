#!/usr/bin/env bash
# NIST CAD-PMI-Testing testcase catalog updater. Downloads only browser HTML catalog, not CAD/STEP/PDF payloads.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/cad/nist-pmi-testcase-catalog"
URL="${NIST_CAD_PMI_CATALOG_URL:-https://pages.nist.gov/CAD-PMI-Testing/models.html}"
mkdir -p "$DEST"
python3 - "$URL" "$DEST/models.html" "$DEST/source-receipt.json" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
url, out_path, receipt_path = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0 (CAD PMI catalog metadata only)'})
with urllib.request.urlopen(req,timeout=60) as r:
    raw=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
out_path.write_bytes(raw)
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'NIST CAD-PMI-Testing test case browser catalog',
  'version':'snapshot-2026-06-19',
  'url':url,
  'final_url':final,
  'sha256':hashlib.sha256(raw).hexdigest(),
  'size_bytes':len(raw),
  'content_type':ctype,
  'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'license':'NIST public data / public domain where applicable',
  'scope':'browser HTML catalog only; CAD/STEP/native model/PDF/report payloads and process/toolpath guidance excluded'
}
receipt_path.write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated NIST CAD PMI catalog: sha={receipt["sha256"]} bytes={len(raw)}')
PY
