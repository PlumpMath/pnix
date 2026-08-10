#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/procurement/samgov-contract-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (SAM.gov API catalog metadata only; no API payload calls)"
declare -A urls=(
  [open_gsa_api]="${SAMGOV_OPEN_GSA_API_URL:-https://open.gsa.gov/api/}"
  [opportunities_doc]="${SAMGOV_OPPORTUNITIES_DOC_URL:-https://open.gsa.gov/api/get-opportunities-public-api/}"
  [opportunities_yml]="${SAMGOV_OPPORTUNITIES_YML_URL:-https://open.gsa.gov/api/get-opportunities-public-api/v1/get-opportunities-v2.yml}"
  [sam_fpds]="${SAMGOV_FPDS_URL:-https://sam.gov/fpds}"
  [sam_contract_data]="${SAMGOV_CONTRACT_DATA_URL:-https://sam.gov/contract-data}"
)
for k in "${!urls[@]}"; do
  curl -fsSL --retry 2 --max-time 45 -A "$UA" "${urls[$k]}" -o "$OUT/$k.${k##*_}" || curl -fsSL --retry 2 --max-time 45 -A "$UA" "${urls[$k]}" -o "$OUT/$k.html"
done
# normalize filenames for predictable generator input
[ -f "$OUT/opportunities_yml.yml" ] || mv "$OUT/opportunities_yml.yml" "$OUT/opportunities.yml" 2>/dev/null || true
[ -f "$OUT/opportunities_yml.yml" ] && mv "$OUT/opportunities_yml.yml" "$OUT/opportunities.yml" || true
[ -f "$OUT/opportunities_yml.html" ] && mv "$OUT/opportunities_yml.html" "$OUT/opportunities.yml" || true
for n in open_gsa_api opportunities_doc sam_fpds sam_contract_data; do
  for ext in api doc fpds data html; do [ -f "$OUT/$n.$ext" ] && mv "$OUT/$n.$ext" "$OUT/$n.html" 2>/dev/null || true; done
done
python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1])
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'samgov-contract-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'files':files,'policy':'SAM.gov/GSA public API docs/catalog metadata only; opportunity/award/entity payloads, API keys, attachments, request logs and procurement advice excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
