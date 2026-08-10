#!/usr/bin/env bash
# USGS Landslide Inventories v3 updater: ScienceBase release/file metadata + references CSV only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/usgs-landslide-inventories-v3"
ITEM_ID="${USGS_LANDSLIDE_V3_ITEM_ID:-671eef1fd34ed0f827ea9f12}"
ITEM_URL="https://www.sciencebase.gov/catalog/item/${ITEM_ID}?format=json"
mkdir -p "$DEST"
python3 - "$ITEM_URL" "$DEST" <<'PY'
import csv, datetime, hashlib, io, json, pathlib, re, sys, urllib.request
item_url, dest = sys.argv[1], pathlib.Path(sys.argv[2])
ua={'User-Agent':'pnix-ingest/1.0 (release metadata only)'}
def fetch(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=120) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
item_raw, final_url, item_ctype = fetch(item_url)
item=(dest/'sciencebase-item.json')
item.write_bytes(item_raw)
obj=json.loads(item_raw)
refs_file=None
for f in obj.get('files') or []:
    if (f.get('name') or '').lower() == 'us_ls_v3_references.csv':
        refs_file=f
        break
if not refs_file:
    raise SystemExit('references CSV not found in ScienceBase files')
refs_url=refs_file.get('downloadUri') or refs_file.get('url')
refs_raw, refs_final, refs_ctype = fetch(refs_url)
refs=(dest/'us_ls_v3_references.csv')
refs.write_bytes(refs_raw)
rows=list(csv.DictReader(io.StringIO(refs_raw.decode('utf-8-sig','replace'))))
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'USGS Landslide Inventories across the United States v3 release metadata',
  'version':'v3.0-february-2025',
  'item_id':obj.get('id'),
  'doi':'10.5066/P14AJF8I',
  'url':item_url,
  'final_url':final_url,
  'item_sha256':hashlib.sha256(item_raw).hexdigest(),
  'item_size_bytes':len(item_raw),
  'item_content_type':item_ctype,
  'references_url':refs_url,
  'references_final_url':refs_final,
  'references_sha256':hashlib.sha256(refs_raw).hexdigest(),
  'references_size_bytes':len(refs_raw),
  'references_content_type':refs_ctype,
  'reference_row_count':len(rows),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'US Government public domain',
  'scope':'ScienceBase release/file metadata + references CSV fingerprints/URLs only; landslide geometry/event rows/ancillary/analyses/safety judgments excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f"updated USGS Landslide v3 metadata: files={len(obj.get('files') or [])} references={len(rows)} item_sha={receipt['item_sha256']} refs_sha={receipt['references_sha256']}")
PY
