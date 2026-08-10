#!/usr/bin/env bash
# NOAA NDBC activestations.xml -> DART station metadata pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NOAA_NDBC_DART_SRC:-$ROOT/ingest/science/noaa-ndbc-dart-stations/activestations.xml}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/noaa-ndbc-dart-stations.generated.px}"
RECEIPT="$ROOT/ingest/science/noaa-ndbc-dart-stations/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing NOAA NDBC activestations XML: $SRC" >&2
  echo "run scripts/update-science-noaa-ndbc-dart-stations.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, hashlib, xml.etree.ElementTree as ET
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
text=src.read_text(encoding='utf-8', errors='replace')
root=ET.fromstring(text)
def boolflag(v):
    return True if v=='y' else False if v=='n' else None
def num(v):
    if v is None or v=='': return None
    try: return float(v)
    except Exception: return v
stations=[]; owners={}
for s in root.findall('station'):
    if s.get('dart')!='y': continue
    item={
      'id':s.get('id'),
      'name':(s.get('name') or '').strip(),
      'lat':num(s.get('lat')),
      'lon':num(s.get('lon')),
      'elev':num(s.get('elev')),
      'owner':s.get('owner'),
      'program':s.get('pgm'),
      'type':s.get('type'),
      'met':boolflag(s.get('met')),
      'currents':boolflag(s.get('currents')),
      'waterquality':boolflag(s.get('waterquality')),
      'dart':boolflag(s.get('dart')),
    }
    stations.append(item)
    owner=item.get('owner') or 'unknown'; owners[owner]=owners.get(owner,0)+1
stations=sorted(stations, key=lambda x:x['id'] or '')
obj={
 'schema':'science.noaa.ndbc_dart_stations.v1',
 'source':{
   'name':'NOAA NDBC DART station metadata',
   'license':'US Government public domain',
   'source_urls':['https://www.ndbc.noaa.gov/activestations.xml','https://www.ndbc.noaa.gov/'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-noaa-ndbc-dart-stations.sh',
   'scope':'NDBC active station metadata filtered to dart=y only; observations/alerts/forecasts/tsunami guidance excluded'
 },
 'source_files':{'xml_sha256':hashlib.sha256(text.encode()).hexdigest()},
 'summary':{'dart_station_count':len(stations),'total_station_count':len(root.findall('station')),'source_created':root.get('created'),'owners':owners,'observations_ingested':False,'alerts_ingested':False},
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
content='# stdlib/lib/corpus/noaa-ndbc-dart-stations.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-noaa-ndbc-dart-stations.sh && scripts/gen-science-noaa-ndbc-dart-stations.sh\n'
content+='# 범위: NOAA NDBC DART station metadata only. observations/alerts/forecasts/tsunami guidance 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: dart_stations={len(stations)} bytes={len(content.encode())}")
PY
