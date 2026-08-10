#!/usr/bin/env bash
# openFDA NDC Directory product/package metadata updater. Bounded identifier rows only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/pharma/fda-ndc-directory-products"
LIMIT="${NDC_LIMIT:-100}"
SKIP="${NDC_SKIP:-0}"
mkdir -p "$DEST"
python3 - "$DEST" "$LIMIT" "$SKIP" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.parse, urllib.request

dest=pathlib.Path(sys.argv[1]); limit=int(sys.argv[2]); skip=int(sys.argv[3])
ua={'User-Agent':'pnix-ingest/1.0 (NDC Directory product/package identifier metadata only)'}
def fetch(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=120) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')

download_raw, download_final, download_ctype = fetch('https://api.fda.gov/download.json')
(dest/'openfda-download.json').write_bytes(download_raw)
params=urllib.parse.urlencode({'limit':limit,'skip':skip})
url='https://api.fda.gov/drug/ndc.json?'+params
raw, final, ctype = fetch(url)
(dest/'ndc.json').write_bytes(raw)
obj=json.loads(raw)
cat=json.loads(download_raw)
ndc_meta=((cat.get('results') or {}).get('drug') or {}).get('ndc') or {}
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'openFDA Drug NDC Directory product/package metadata',
 'version':f'snapshot-2026-06-19-limit-{limit}-skip-{skip}',
 'url':url,
 'final_url':final,
 'sha256':hashlib.sha256(raw).hexdigest(),
 'size_bytes':len(raw),
 'content_type':ctype,
 'download_catalog_url':'https://api.fda.gov/download.json',
 'download_catalog_sha256':hashlib.sha256(download_raw).hexdigest(),
 'download_catalog_content_type':download_ctype,
 'openfda_export_date':ndc_meta.get('export_date'),
 'openfda_total_records':ndc_meta.get('total_records'),
 'api_total_records':((obj.get('meta') or {}).get('results') or {}).get('total'),
 'limit':limit,
 'skip':skip,
 'returned_product_count':len(obj.get('results') or []),
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'openFDA public domain / CC0 public data policy',
 'scope':'bounded NDC product/package identifier metadata only; label text, active ingredient strength values, package description prose, prescribing/safety guidance, adverse-event/enforcement payloads, and graph/mirror wiring excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated NDC Directory products: limit={limit} skip={skip} rows={receipt["returned_product_count"]} total={receipt["api_total_records"]} sha={receipt["sha256"]}')
PY
