#!/usr/bin/env bash
# USGS 3DEP Elevation Index ArcGIS REST metadata snapshot.
# Only service/layer JSON metadata and returnCountOnly counts are fetched; no feature rows/geometries/raster payloads.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${USGS_3DEP_DEST:-$ROOT/ingest/science/usgs-3dep-index}"
BASE="${USGS_3DEP_BASE:-https://index.nationalmap.gov/arcgis/rest/services/3DEPElevationIndex/MapServer}"
mkdir -p "$DEST/layers"
python3 - "$BASE" "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
base=sys.argv[1].rstrip('/'); dest=pathlib.Path(sys.argv[2])
def fetch_json(url):
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-usgs-3dep-index-ingest'})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)
def save_json(path,obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    data=json.dumps(obj,ensure_ascii=False,indent=2,sort_keys=True)+'\n'
    path.write_text(data,encoding='utf-8')
    return hashlib.sha256(data.encode()).hexdigest(), len(data.encode())
service=fetch_json(base+'?f=pjson')
svc_sha,svc_size=save_json(dest/'service.json',service)
files=[{'relative_path':'service.json','sha256':svc_sha,'size_bytes':svc_size,'role':'service_metadata'}]
layers=[]
for l in service.get('layers',[]):
    lid=l.get('id')
    meta=fetch_json(f'{base}/{lid}?f=pjson')
    count=None
    if meta.get('type')=='Feature Layer':
        q=f'{base}/{lid}/query?where=1%3D1&returnCountOnly=true&f=pjson'
        try:
            count=fetch_json(q).get('count')
        except Exception:
            count=None
    meta['_pnix_returnCountOnly_count']=count
    sha,size=save_json(dest/'layers'/f'{lid}.json',meta)
    files.append({'relative_path':f'layers/{lid}.json','sha256':sha,'size_bytes':size,'role':'layer_metadata','layer_id':lid})
    layers.append({'id':lid,'name':meta.get('name'),'type':meta.get('type'),'geometryType':meta.get('geometryType'),'field_count':len(meta.get('fields') or []),'return_count_only_count':count})
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'USGS The National Map 3DEP Elevation Index service metadata',
  'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),
  'service_url':base,
  'source_urls':['https://www.usgs.gov/3d-elevation-program','https://www.usgs.gov/faqs/there-api-accessing-national-map-data',base],
  'license':'USGS public domain / courtesy credit requested',
  'scope':'service/layer metadata and returnCountOnly counts only; no feature rows, geometry, raster, derivative products, engineering/safety judgments, or graph/mirror wiring',
  'service_sha256':svc_sha,
  'layer_count':len(layers),
  'feature_layer_count':sum(1 for x in layers if x.get('type')=='Feature Layer'),
  'files':files,
  'layers':layers,
}
save_json(dest/'source-receipt.json',receipt)
print(f'downloaded USGS 3DEP index metadata: layers={len(layers)} feature_layers={receipt["feature_layer_count"]} -> {dest}')
PY
