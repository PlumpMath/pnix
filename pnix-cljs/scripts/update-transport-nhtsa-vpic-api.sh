#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/transport/nhtsa-vpic-api"
MAKE_ID="${NHTSA_VPIC_MAKE_ID:-440}"
MODEL_YEAR="${NHTSA_VPIC_MODEL_YEAR:-2024}"
mkdir -p "$DST"
python3 - "$DST" "$MAKE_ID" "$MODEL_YEAR" <<'PY'
import json, pathlib, sys, urllib.request, hashlib, datetime
root=pathlib.Path(sys.argv[1]); make_id=sys.argv[2]; year=sys.argv[3]
endpoints={
  'all_makes': 'https://vpic.nhtsa.dot.gov/api/vehicles/GetAllMakes?format=json',
  'manufacturers_page_1': 'https://vpic.nhtsa.dot.gov/api/vehicles/getallmanufacturers?format=json&page=1',
  'vehicle_types_for_make': f'https://vpic.nhtsa.dot.gov/api/vehicles/GetVehicleTypesForMakeId/{make_id}?format=json',
  'models_for_make_year': f'https://vpic.nhtsa.dot.gov/api/vehicles/GetModelsForMakeIdYear/makeId/{make_id}/modelyear/{year}?format=json'
}
files=[]
for name,url in endpoints.items():
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0','Accept':'application/json'})
    with urllib.request.urlopen(req,timeout=60) as r:
        b=r.read()
    json.loads(b)
    p=root/(name+'.json')
    p.write_bytes(b)
    files.append({'path':p.name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'url':url})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'nhtsa-vpic-api','source_name':'NHTSA vPIC Vehicle API','license_id':'US-PD / US federal public API metadata','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'make_id':make_id,'model_year':year,'files':files,'source_urls':list(endpoints.values()),'policy':'Bounded make/manufacturer/model/type metadata only. Exclude VIN decode logs, individual VINs, owner/person data, repair/diagnostic advice, legal/compliance decisions, runtime dependency, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json endpoints={len(endpoints)} make_id={make_id} model_year={year}')
PY
