#!/usr/bin/env bash
# NOAA CO-OPS station metadata JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NOAA_COOPS_STATIONS_SRC:-$ROOT/ingest/science/noaa-coops-stations/stations.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/noaa-coops-stations.generated.px}"
RECEIPT="$ROOT/ingest/science/noaa-coops-stations/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing NOAA CO-OPS stations JSON: $SRC" >&2
  echo "run scripts/update-science-noaa-coops-stations.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, hashlib
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
text=src.read_text(encoding='utf-8', errors='replace')
data=json.loads(text)
def endpoint_ref(v):
    if isinstance(v,dict) and 'self' in v: return v['self']
    return None
stations=[]
for s in data.get('stations',[]):
    endpoints={k:endpoint_ref(s.get(k)) for k in ['details','sensors','floodlevels','datums','supersededdatums','tidepredoffsets','products','disclaimers'] if endpoint_ref(s.get(k))}
    stations.append({
      'id': str(s.get('id','')),
      'name': s.get('name'),
      'state': s.get('state'),
      'lat': s.get('lat'),
      'lng': s.get('lng'),
      'tidal': s.get('tidal'),
      'greatlakes': s.get('greatlakes'),
      'shefcode': s.get('shefcode'),
      'type': s.get('type'),
      'timezonecorr': s.get('timezonecorr'),
      'timemeridian': s.get('timemeridian'),
      'reference_id': s.get('reference_id'),
      'affiliations': s.get('affiliations'),
      'portscode': s.get('portscode'),
      'products': s.get('products'),
      'metadata_endpoints': endpoints,
    })
stations=sorted(stations, key=lambda x:x['id'])
states={}
for s in stations:
    k=s.get('state') or 'unknown'; states[k]=states.get(k,0)+1
obj={
 'schema':'science.noaa.coops_stations.v1',
 'source':{
   'name':'NOAA CO-OPS Tides and Currents station metadata',
   'license':'US Government public domain',
   'source_urls':['https://api.tidesandcurrents.noaa.gov/mdapi/prod/webapi/stations.json','https://tidesandcurrents.noaa.gov/'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-noaa-coops-stations.sh',
   'scope':'base station registry metadata only; observations/predictions/forecasts/alerts/navigation decisions excluded'
 },
 'source_files':{'json_sha256':hashlib.sha256(text.encode()).hexdigest()},
 'summary':{'station_count':len(stations),'states':states,'observations_ingested':False,'predictions_ingested':False},
 'stations':stations,
}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/noaa-coops-stations.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-noaa-coops-stations.sh && scripts/gen-science-noaa-coops-stations.sh\n'
content+='# 범위: NOAA CO-OPS base station metadata only. observations/predictions/forecasts/alerts/navigation 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: stations={len(stations)} bytes={len(content.encode())}")
PY
