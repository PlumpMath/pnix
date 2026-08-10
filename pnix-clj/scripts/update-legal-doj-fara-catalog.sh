#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/legal/doj-fara-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (DOJ FARA catalog metadata only; no filing/search payloads)"
declare -A urls=(
  [doj_fara]="${DOJ_FARA_PAGE_URL:-https://www.justice.gov/nsd-fara}"
  [doj_efile]="${DOJ_FARA_EFILE_URL:-https://www.justice.gov/nsd-fara/fara-efile}"
  [efile_search]="${DOJ_FARA_SEARCH_URL:-https://efile.fara.gov/ords/fara/f?p=135:10}"
  [datagov_page]="${DOJ_FARA_DATAGOV_PAGE_URL:-https://catalog.data.gov/dataset/foreign-agent-registration-act-efile-system}"
)
for k in "${!urls[@]}"; do
  curl -fsSL --retry 2 --max-time 45 -A "$UA" "${urls[$k]}" -o "$OUT/$k.html"
done
curl -fsSL --retry 2 --max-time 45 -A "$UA" "${DOJ_FARA_CKAN_URL:-https://catalog.data.gov/api/3/action/package_show?id=foreign-agent-registration-act-efile-system}" -o "$OUT/datagov-package.json"
python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1])
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'doj-fara-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'files':files,'policy':'DOJ FARA/Data.gov catalog metadata only; FARA registrant/filing/search payloads, PDFs, legal guidance prose, credentials and graph wiring excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
