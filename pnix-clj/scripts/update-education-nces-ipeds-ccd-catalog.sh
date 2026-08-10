#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/education/nces-ipeds-ccd-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
IPEDS_URL="${NCES_IPEDS_USE_DATA_URL:-https://nces.ed.gov/ipeds/use-the-data}"
CCD_FILES_URL="${NCES_CCD_FILES_URL:-https://nces.ed.gov/ccd/files.asp}"
CCD_SCHOOL_URL="${NCES_CCD_SCHOOL_URL:-https://nces.ed.gov/ccd/pubschuniv.asp}"
CCD_AGENCY_URL="${NCES_CCD_AGENCY_URL:-https://nces.ed.gov/ccd/pubagency.asp}"
curl -fsSL -A "$UA" "$IPEDS_URL" -o "$OUT/ipeds-use-data.html"
curl -fsSL -A "$UA" "$CCD_FILES_URL" -o "$OUT/ccd-files.html"
curl -fsSL -A "$UA" "$CCD_SCHOOL_URL" -o "$OUT/ccd-school-universe.html"
curl -fsSL -A "$UA" "$CCD_AGENCY_URL" -o "$OUT/ccd-agency-universe.html"
python3 - "$OUT" "$IPEDS_URL" "$CCD_FILES_URL" "$CCD_SCHOOL_URL" "$CCD_AGENCY_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'nces-ipeds-ccd-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'NCES catalog/download-page metadata only; institution/school/student/statistical payload rows excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
