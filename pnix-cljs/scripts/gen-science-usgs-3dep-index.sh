#!/usr/bin/env bash
# USGS 3DEP Elevation Index metadata JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USGS_3DEP_SRC:-$ROOT/ingest/science/usgs-3dep-index}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-3dep-index.generated.px}"
if [[ ! -f "$SRC/service.json" ]]; then
  echo "missing USGS 3DEP metadata snapshot: $SRC" >&2
  echo "run scripts/update-science-usgs-3dep-index.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
service=json.loads((src/'service.json').read_text(encoding='utf-8'))
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
layers=[]; fields=[]
for lp in sorted((src/'layers').glob('*.json'), key=lambda p:int(p.stem)):
    m=json.loads(lp.read_text(encoding='utf-8'))
    lid=m.get('id')
    sublayers=[{'id':x.get('id'),'name':x.get('name')} for x in (m.get('subLayers') or [])]
    layers.append({
      'layer_id':lid,
      'name':m.get('name'),
      'type':m.get('type'),
      'parent_layer_id':(m.get('parentLayer') or {}).get('id') if isinstance(m.get('parentLayer'),dict) else None,
      'geometry_type':m.get('geometryType'),
      'default_visibility':m.get('defaultVisibility'),
      'min_scale':m.get('minScale'),
      'max_scale':m.get('maxScale'),
      'display_field':m.get('displayField'),
      'field_count':len(m.get('fields') or []),
      'sublayers':sublayers,
      'return_count_only_count':m.get('_pnix_returnCountOnly_count'),
      'feature_rows_ingested':False,
      'geometry_ingested':False,
    })
    for f in m.get('fields') or []:
        fields.append({
          'layer_id':lid,
          'layer_name':m.get('name'),
          'name':f.get('name'),
          'type':f.get('type'),
          'alias':f.get('alias'),
          'length':f.get('length'),
          'nullable':f.get('nullable'),
          'editable':f.get('editable'),
          'domain_type':(f.get('domain') or {}).get('type') if isinstance(f.get('domain'),dict) else None,
        })
obj={
 'schema':'science.usgs_3dep_index.v1',
 'source':{
   'name':'USGS The National Map 3DEP Elevation Index service metadata',
   'license':'USGS public domain / courtesy credit requested',
   'source_urls':['https://www.usgs.gov/3d-elevation-program','https://index.nationalmap.gov/arcgis/rest/services/3DEPElevationIndex/MapServer'],
   'receipt':receipt,
   'generator':'scripts/gen-science-usgs-3dep-index.sh',
   'scope':'ArcGIS REST service/layer/field/count metadata only; no feature rows, geometry, raster payloads, derivative products, engineering/safety judgments, or graph/mirror wiring'
 },
 'service':{
   'service_url':receipt.get('service_url'),
   'current_version':service.get('currentVersion'),
   'map_name':service.get('mapName'),
   'spatial_reference_wkid':(service.get('spatialReference') or {}).get('wkid'),
   'units':service.get('units'),
   'layer_count':len(service.get('layers') or []),
   'supports_query_domains':service.get('supportsQueryDomains'),
   'single_fused_map_cache':service.get('singleFusedMapCache'),
 },
 'summary':{
   'layer_count':len(layers),
   'feature_layer_count':sum(1 for x in layers if x.get('type')=='Feature Layer'),
   'field_count':len(fields),
   'return_count_only_total':sum((x.get('return_count_only_count') or 0) for x in layers),
   'raster_payloads_ingested':False,
   'feature_rows_ingested':False,
   'geometry_ingested':False,
   'precise_tile_footprints_ingested':False,
   'derivative_products_ingested':False,
   'engineering_or_safety_judgment_ingested':False,
   'operational_guidance_ingested':False,
   'mirror_graph_wiring':False,
 },
 'layers':layers,
 'fields':fields,
}
def pnix(v, indent=0):
    import json
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
content='# stdlib/lib/corpus/usgs-3dep-index.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-3dep-index.sh && scripts/gen-science-usgs-3dep-index.sh\n'
content+='# 범위: USGS 3DEP Elevation Index service/layer/field/count metadata only. raster/features/geometry/engineering/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f'generated {out}: layers={len(layers)} fields={len(fields)} bytes={len(content.encode())}')
PY
