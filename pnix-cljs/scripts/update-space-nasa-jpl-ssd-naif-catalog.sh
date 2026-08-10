#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NASA_JPL_CATALOG_DEST:-$ROOT/ingest/space/nasa-jpl-ssd-naif-catalog}"
mkdir -p "$DEST/raw/ssd" "$DEST/raw/naif"
python3 - "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, re, urllib.request, sys
DEST=pathlib.Path(sys.argv[1]); UA='pnix-nasa-jpl-catalog-ingest/1.0 (catalog metadata only; no API payloads)'
PAGES=[
 ('ssd','index','https://ssd-api.jpl.nasa.gov/'),
 ('ssd','sbdb','https://ssd-api.jpl.nasa.gov/doc/sbdb.html'),
 ('ssd','sbdb_query','https://ssd-api.jpl.nasa.gov/doc/sbdb_query.html'),
 ('ssd','cad','https://ssd-api.jpl.nasa.gov/doc/cad.html'),
 ('ssd','horizons','https://ssd-api.jpl.nasa.gov/doc/horizons.html'),
 ('ssd','fireball','https://ssd-api.jpl.nasa.gov/doc/fireball.html'),
 ('naif','rules','https://naif.jpl.nasa.gov/naif/rules.html'),
 ('naif','generic_lsk','https://naif.jpl.nasa.gov/pub/naif/generic_kernels/lsk/'),
 ('naif','generic_pck','https://naif.jpl.nasa.gov/pub/naif/generic_kernels/pck/'),
 ('naif','generic_spk','https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/'),
]
files=[]
for family,name,url in PAGES:
    req=urllib.request.Request(url,headers={'User-Agent':UA})
    try:
        raw=urllib.request.urlopen(req,timeout=60).read()
        status=200
    except Exception as e:
        raw=(f'ERROR: {e}\n').encode(); status=0
    rel=f'raw/{family}/{name}.html'
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    files.append({'family':family,'name':name,'url':url,'relative_path':rel,'status':status,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'NASA/JPL SSD API and NAIF public catalog metadata','retrieved_at':dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':[x[2] for x in PAGES],'license':'NASA/JPL public documentation + NAIF redistribution-permitted public data metadata','scope':'official page/directory catalog metadata only; no API result payloads, ephemeris/kernel payloads, close-approach/hazard rows, Horizons results, mission kernels, trajectory data, operational guidance, execution, or mirror/graph wiring','files':files}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded NASA/JPL catalog pages: files={len(files)} -> {DEST}')
PY
