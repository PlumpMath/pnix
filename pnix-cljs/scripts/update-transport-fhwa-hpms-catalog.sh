#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/transport/fhwa-hpms-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
PAGE_URL="${FHWA_HPMS_PAGE_URL:-https://www.fhwa.dot.gov/policyinformation/hpms.cfm}"
SHAPEFILES_URL="${FHWA_HPMS_SHAPEFILES_URL:-https://www.fhwa.dot.gov/policyinformation/hpms/shapefiles.cfm}"
DATASET_PAGE_URL="${FHWA_HPMS_DATASET_PAGE_URL:-https://catalog.data.gov/dataset/highway-performance-monitoring-system-hpms-2024}"
curl -fsSL -A "$UA" "$PAGE_URL" -o "$OUT/fhwa-hpms.html"
curl -fsSL -A "$UA" "$SHAPEFILES_URL" -o "$OUT/fhwa-hpms-shapefiles.html"
curl -fsSL -A "$UA" "$DATASET_PAGE_URL" -o "$OUT/data-gov-hpms-2024.html"
python3 - "$OUT" "$PAGE_URL" "$SHAPEFILES_URL" "$DATASET_PAGE_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'fhwa-hpms-catalog','retrieved_at_utc':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z','source_urls':urls,'files':files,'policy':'catalog metadata only; road segment payloads/geometries/values/prose bodies excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
