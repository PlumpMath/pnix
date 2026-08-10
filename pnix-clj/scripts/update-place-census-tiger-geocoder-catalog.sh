#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CENSUS_TIGER_DEST:-$ROOT/ingest/place/census-tiger-geocoder}"
UA="${CENSUS_TIGER_USER_AGENT:-pnix-ingest/0.1 (Census TIGERweb+Geocoder metadata catalog)}"
FOLDERS="${CENSUS_TIGER_FOLDERS:-TIGERweb Census2020 Generalized_ACS2025}"
mkdir -p "$DEST/raw/tigerweb" "$DEST/raw/geocoder"
python3 - <<'PY' "$DEST" "$UA" "$FOLDERS"
import hashlib,json,pathlib,sys,time,urllib.parse,urllib.request
root=pathlib.Path(sys.argv[1]); ua=sys.argv[2]; folders=sys.argv[3].split()
headers={'User-Agent':ua}
files=[]
def fetch(url,path):
    req=urllib.request.Request(url,headers=headers)
    with urllib.request.urlopen(req,timeout=60) as r:
        data=r.read()
    path.parent.mkdir(parents=True,exist_ok=True); path.write_bytes(data)
    files.append({'path':str(path.relative_to(root)),'url':url,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
    return json.loads(data.decode('utf-8'))
root_url='https://tigerweb.geo.census.gov/arcgis/rest/services?f=pjson'
fetch(root_url,root/'raw/tigerweb/root-services.json')
service_names=[]
for folder in folders:
    listing=fetch(f'https://tigerweb.geo.census.gov/arcgis/rest/services/{urllib.parse.quote(folder)}?f=pjson', root/f'raw/tigerweb/{folder}-services.json')
    for svc in listing.get('services',[]):
        name=svc.get('name'); typ=svc.get('type')
        if typ=='MapServer' and name and name not in service_names:
            service_names.append(name)
for name in service_names:
    safe=name.replace('/','__')
    fetch(f'https://tigerweb.geo.census.gov/arcgis/rest/services/{urllib.parse.quote(name,safe="/")}/MapServer?f=pjson', root/f'raw/tigerweb/{safe}.mapserver.json')
bench=fetch('https://geocoding.geo.census.gov/geocoder/benchmarks?format=json', root/'raw/geocoder/benchmarks.json')
for b in bench.get('benchmarks',[]):
    name=b.get('benchmarkName') or b.get('id') or ''
    if 'LUCA' in name.upper():
        continue
    bid=b.get('id') or name
    safe=str(name or bid).replace('/','_')
    fetch(f'https://geocoding.geo.census.gov/geocoder/vintages?benchmark={urllib.parse.quote(str(bid))}&format=json', root/f'raw/geocoder/vintages-{safe}.json')
(root/'source-receipt.json').write_text(json.dumps({'schema':'place.census_tiger_geocoder.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / U.S. Census Bureau public information','folders':folders,'files':files,'excluded':['TIGER/Line geometry files','coordinates/extents','address geocoder query results','batch geocoder inputs/outputs','Title-13 LUCA address files','Census microdata/IPUMS/PUMS','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched {len(files)} files into {root}')
PY
