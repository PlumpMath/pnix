#!/usr/bin/env bash
# NASA Exoplanet Archive pscomppars updater.
# Downloads selected TAP CSV columns only. No prose, light curves, spectra, images, prediction, or graph/mirror wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/science/nasa-exoplanet-archive"
mkdir -p "$DEST"
QUERY="select pl_name,hostname,discoverymethod,disc_year,disc_facility,pl_orbper,pl_orbsmax,pl_orbeccen,pl_rade,pl_bmasse,pl_eqt,st_teff,st_rad,st_mass,sy_dist,sy_snum,sy_pnum from pscomppars order by pl_name"
CSV="$DEST/pscomppars-selected.csv"
python3 - "$QUERY" "$CSV" "$DEST/source-receipt.json" <<'PY'
import sys, urllib.parse, urllib.request, hashlib, json, datetime, os
query,csv_path,receipt_path=sys.argv[1:]
base='https://exoplanetarchive.ipac.caltech.edu/TAP/sync'
url=base+'?'+urllib.parse.urlencode({'query':query,'format':'csv'})
req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/1.0'})
with urllib.request.urlopen(req, timeout=120) as r:
    data=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
open(csv_path,'wb').write(data)
sha=hashlib.sha256(data).hexdigest()
row_count=max(0, data.decode('utf-8','replace').count('\n')-1)
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'NASA Exoplanet Archive pscomppars selected columns',
  'version':'snapshot-2026-06-19',
  'url':url,
  'final_url':final,
  'sha256':sha,
  'size_bytes':len(data),
  'content_type':ctype,
  'row_count':row_count,
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'NASA public data / acknowledgment required',
  'scope':'selected confirmed planet composite parameter table fields only; no prose/light curves/spectra/images/time-series/prediction/graph wiring'
}, open(receipt_path,'w'), indent=2, ensure_ascii=False)
print(f'updated NASA Exoplanet Archive pscomppars snapshot: rows={row_count} sha={sha} bytes={len(data)}')
PY
