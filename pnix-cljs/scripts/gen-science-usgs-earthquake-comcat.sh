#!/usr/bin/env bash
# USGS ComCat GeoJSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USGS_COMCAT_SRC:-$ROOT/ingest/science/usgs-earthquake-comcat/comcat-events.geojson}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-earthquake-comcat.generated.px}"
RECEIPT="$ROOT/ingest/science/usgs-earthquake-comcat/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing USGS ComCat GeoJSON: $SRC" >&2
  echo "run scripts/update-science-usgs-earthquake-comcat.sh first" >&2
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
events=[]; by_type={}; by_magtype={}
for f in data.get('features',[]):
    p=f.get('properties') or {}; g=f.get('geometry') or {}; coords=g.get('coordinates') or [None,None,None]
    ev={
      'id': f.get('id'),
      'time_ms': p.get('time'),
      'updated_ms': p.get('updated'),
      'mag': p.get('mag'),
      'mag_type': p.get('magType'),
      'type': p.get('type'),
      'status': p.get('status'),
      'place': p.get('place'),
      'longitude': coords[0] if len(coords)>0 else None,
      'latitude': coords[1] if len(coords)>1 else None,
      'depth_km': coords[2] if len(coords)>2 else None,
      'net': p.get('net'),
      'code': p.get('code'),
      'url': p.get('url'),
      'detail_url': p.get('detail'),
    }
    events.append(ev)
    by_type[ev.get('type') or 'unknown']=by_type.get(ev.get('type') or 'unknown',0)+1
    by_magtype[ev.get('mag_type') or 'unknown']=by_magtype.get(ev.get('mag_type') or 'unknown',0)+1
events=sorted(events, key=lambda x:(x.get('time_ms') or 0, x.get('id') or ''))
obj={
 'schema':'science.usgs.earthquake_comcat.events.v1',
 'source':{
   'name':'USGS Earthquake Catalog / ComCat historical event metadata',
   'license':'US Government public domain',
   'source_urls':['https://earthquake.usgs.gov/fdsnws/event/1/','https://earthquake.usgs.gov/fdsnws/event/1/query'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-usgs-earthquake-comcat.sh',
   'scope':'bounded historical event metadata only; alerts/PAGER/ShakeMap/felt reports/tsunami warning/forecast/response guidance excluded'
 },
 'source_files':{'geojson_sha256':hashlib.sha256(text.encode()).hexdigest()},
 'summary':{'event_count':len(events),'events_by_type':by_type,'events_by_mag_type':by_magtype,'alert_fields_ingested':False,'pager_shakemap_ingested':False,'felt_reports_ingested':False},
 'events':events,
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
content='# stdlib/lib/corpus/usgs-earthquake-comcat.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-earthquake-comcat.sh && scripts/gen-science-usgs-earthquake-comcat.sh\n'
content+='# 범위: bounded historical USGS ComCat event metadata only. alerts/PAGER/ShakeMap/felt/tsunami/forecast/response 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: events={len(events)} bytes={len(content.encode())}")
PY
