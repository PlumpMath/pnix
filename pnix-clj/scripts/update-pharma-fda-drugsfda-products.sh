#!/usr/bin/env bash
# openFDA Drugs@FDA product metadata updater. Bounded application/product rows only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/pharma/fda-drugsfda-products"
LIMIT="${DRUGSFDA_LIMIT:-100}"
SKIP="${DRUGSFDA_SKIP:-0}"
mkdir -p "$DEST"
python3 - "$DEST" "$LIMIT" "$SKIP" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.parse, urllib.request
dest=pathlib.Path(sys.argv[1]); limit=int(sys.argv[2]); skip=int(sys.argv[3])
ua={'User-Agent':'pnix-ingest/1.0 (Drugs@FDA product metadata only)'}
def fetch(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=120) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
download_raw, download_final, download_ctype = fetch('https://api.fda.gov/download.json')
(dest/'openfda-download.json').write_bytes(download_raw)
params=urllib.parse.urlencode({'limit':limit,'skip':skip})
url='https://api.fda.gov/drug/drugsfda.json?'+params
raw, final, ctype = fetch(url)
(dest/'drugsfda.json').write_bytes(raw)
obj=json.loads(raw)
cat=json.loads(download_raw)
drugsfda_meta=((cat.get('results') or {}).get('drug') or {}).get('drugsfda') or {}
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'openFDA Drugs@FDA application/product metadata',
 'version':f'snapshot-2026-06-19-limit-{limit}-skip-{skip}',
 'url':url,
 'final_url':final,
 'sha256':hashlib.sha256(raw).hexdigest(),
 'size_bytes':len(raw),
 'content_type':ctype,
 'download_catalog_url':'https://api.fda.gov/download.json',
 'download_catalog_sha256':hashlib.sha256(download_raw).hexdigest(),
 'download_catalog_content_type':download_ctype,
 'openfda_export_date':drugsfda_meta.get('export_date'),
 'openfda_total_records':drugsfda_meta.get('total_records'),
 'api_total_records':((obj.get('meta') or {}).get('results') or {}).get('total'),
 'limit':limit,
 'skip':skip,
 'returned_application_count':len(obj.get('results') or []),
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'openFDA public domain / public data policy',
 'scope':'bounded Drugs@FDA application/product metadata only; submissions/application_docs/labels/dosage-strength/prescribing/safety/adverse-event/recall payloads excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated Drugs@FDA products: limit={limit} skip={skip} rows={receipt["returned_application_count"]} total={receipt["api_total_records"]} sha={receipt["sha256"]}')
PY
