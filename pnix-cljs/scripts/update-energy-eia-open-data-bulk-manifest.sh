#!/usr/bin/env bash
# EIA Open Data bulk manifest updater. Manifest metadata only; bulk zip payloads are not downloaded.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/energy/eia-open-data-bulk-manifest"
mkdir -p "$DEST"
python3 - "$DEST" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request

dest=pathlib.Path(sys.argv[1])
ua={'User-Agent':'pnix-ingest/1.0 (EIA Open Data bulk manifest metadata only)'}
def fetch(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=120) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
manifest_url='https://api.eia.gov/bulk/manifest.txt'
open_data_url='https://www.eia.gov/opendata/bulkfiles.php'
manifest_raw, manifest_final, manifest_ctype = fetch(manifest_url)
open_raw, open_final, open_ctype = fetch(open_data_url)
(dest/'manifest.txt').write_bytes(manifest_raw)
(dest/'bulkfiles.html').write_bytes(open_raw)
obj=json.loads(manifest_raw)
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'EIA Open Data bulk manifest',
 'version':'snapshot-2026-06-19',
 'manifest_url':manifest_url,
 'manifest_final_url':manifest_final,
 'manifest_sha256':hashlib.sha256(manifest_raw).hexdigest(),
 'manifest_size_bytes':len(manifest_raw),
 'manifest_content_type':manifest_ctype,
 'bulkfiles_url':open_data_url,
 'bulkfiles_final_url':open_final,
 'bulkfiles_sha256':hashlib.sha256(open_raw).hexdigest(),
 'bulkfiles_content_type':open_ctype,
 'dataset_count':len((obj.get('dataset') or {})),
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'EIA public domain / acknowledgment requested',
 'scope':'bulk manifest dataset metadata only; bulk zip payloads, time-series values, operational dispatch guidance, security-sensitive facility detail, forecast/trading advice, and graph/mirror wiring excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated EIA bulk manifest: datasets={receipt["dataset_count"]} bytes={len(manifest_raw)} sha={receipt["manifest_sha256"]}')
PY
