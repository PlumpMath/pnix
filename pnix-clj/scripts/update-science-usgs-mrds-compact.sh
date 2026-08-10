#!/usr/bin/env bash
# USGS MRDS flattened CSV updater. Downloads official CSV zip; generator performs bounded metadata-only extraction.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usgs-mrds-compact"
URL="${USGS_MRDS_CSV_ZIP_URL:-https://mrdata.usgs.gov/mrds/mrds-csv.zip}"
mkdir -p "$DEST"
ZIP="$DEST/mrds-csv.zip"
python3 - "$URL" "$ZIP" "$DEST/source-receipt.json" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request, zipfile
url, zip_path, receipt_path = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0 (MRDS metadata only)'})
with urllib.request.urlopen(req,timeout=240) as r:
    raw=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
zip_path.write_bytes(raw)
sha=hashlib.sha256(raw).hexdigest()
with zipfile.ZipFile(zip_path) as z:
    names=z.namelist()
    info={n:z.getinfo(n).file_size for n in names}
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'USGS Mineral Resources Data System (MRDS) flattened CSV',
  'version':'snapshot-2026-06-19-mrds-csv',
  'url':url,
  'final_url':final,
  'sha256':sha,
  'size_bytes':len(raw),
  'content_type':ctype,
  'zip_entries':info,
  'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'license':'US Government public domain',
  'scope':'official mrds-csv.zip downloaded; generator stores bounded metadata-only records and excludes production/resource/geologic prose/full reports/geometry/extraction guidance'
}
receipt_path.write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated USGS MRDS CSV zip: sha={sha} bytes={len(raw)} entries={info}')
PY
