#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/transport/nhtsa-recalls-api"
MAKE="${NHTSA_RECALLS_MAKE:-Honda}"
MODEL="${NHTSA_RECALLS_MODEL:-Accord}"
YEAR="${NHTSA_RECALLS_MODEL_YEAR:-2020}"
mkdir -p "$DST"
python3 - "$DST" "$MAKE" "$MODEL" "$YEAR" <<'PY'
import json, pathlib, sys, urllib.parse, urllib.request, hashlib, datetime
root=pathlib.Path(sys.argv[1]); make=sys.argv[2]; model=sys.argv[3]; year=sys.argv[4]
url='https://api.nhtsa.gov/recalls/recallsByVehicle?'+urllib.parse.urlencode({'make':make,'model':model,'modelYear':year})
req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0','Accept':'application/json'})
with urllib.request.urlopen(req,timeout=45) as r:
    b=r.read()
json.loads(b)
(root/'recalls_by_vehicle.json').write_bytes(b)
files=[{'path':'recalls_by_vehicle.json','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'url':url}]
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'nhtsa-recalls-api','source_name':'NHTSA Recalls API','license_id':'US-PD / US federal public API metadata','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'make':make,'model':model,'model_year':year,'files':files,'source_urls':[url],'policy':'Bounded recall campaign metadata only. Exclude VINs, complaints, ODI narratives, summary/consequence/remedy/notes prose bodies, repair/diagnostic advice, legal/compliance decisions, runtime dependency, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json make={make} model={model} year={year}')
PY
