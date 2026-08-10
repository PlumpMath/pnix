#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${MP_API_CATALOG_IN:-$ROOT/ingest/materials/materials-project-api-catalog/openapi.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/materials-project-api-catalog.generated.px}"
PATH_LIMIT="${MP_API_PATH_LIMIT:-80}"
SCHEMA_LIMIT="${MP_API_SCHEMA_LIMIT:-180}"
python3 - "$IN" "$OUT" "$PATH_LIMIT" "$SCHEMA_LIMIT" <<'PY'
import json, sys
from pathlib import Path
inp=Path(sys.argv[1]); out=Path(sys.argv[2]); path_limit=int(sys.argv[3]); schema_limit=int(sys.argv[4])
def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
j=json.load(open(inp))
paths=[]
for path,ops in sorted(j.get('paths',{}).items())[:path_limit]:
    methods=[]
    for method,op in sorted(ops.items()):
        if not isinstance(op,dict): continue
        params=[]
        for p in op.get('parameters',[])[:80]:
            schema=p.get('schema',{}) if isinstance(p,dict) else {}
            params.append({'name':p.get('name',''),'where':p.get('in',''),'required':bool(p.get('required',False)),'type':schema.get('type','') if isinstance(schema,dict) else ''})
        responses=[]
        for code,r in sorted(op.get('responses',{}).items())[:20]:
            content=r.get('content',{}) if isinstance(r,dict) else {}
            refs=[]
            for mt,body in list(content.items())[:8]:
                schema=body.get('schema',{}) if isinstance(body,dict) else {}
                ref=schema.get('$ref','') or schema.get('items',{}).get('$ref','') if isinstance(schema,dict) else ''
                if ref: refs.append({'media_type':mt,'schema_ref':ref})
            responses.append({'status':str(code),'schema_refs':refs})
        methods.append({'method':method,'operation_id':op.get('operationId',''),'tags':[str(x) for x in op.get('tags',[])[:16]],'parameters':params,'responses':responses})
    paths.append({'path':path,'methods':methods})
schemas=[]
for name,s in sorted(j.get('components',{}).get('schemas',{}).items())[:schema_limit]:
    props=[]
    for pn_,pv in sorted((s.get('properties') or {}).items())[:120]:
        typ=pv.get('type','') if isinstance(pv,dict) else ''
        ref=pv.get('$ref','') if isinstance(pv,dict) else ''
        item_ref=pv.get('items',{}).get('$ref','') if isinstance(pv,dict) and isinstance(pv.get('items'),dict) else ''
        props.append({'name':pn_,'type':typ,'ref':ref or item_ref})
    schemas.append({'name':name,'type':s.get('type','') if isinstance(s,dict) else '','property_count_total':len(s.get('properties') or {}),'properties':props})
seed={'schema':'materials.materials_project_api_catalog.v1','source':{'name':'Materials Project API OpenAPI catalog metadata','license':'public API documentation metadata','version':j.get('info',{}).get('version',''),'url':'https://api.materialsproject.org/openapi.json'},'summary':{'paths_stored':len(paths),'paths_available':len(j.get('paths',{})),'schemas_stored':len(schemas),'schemas_available':len(j.get('components',{}).get('schemas',{})),'database_rows_ingested':False,'material_structures_ingested':False,'composition_values_ingested':False,'computed_properties_ingested':False,'contrib_payloads_ingested':False,'descriptions_or_examples_ingested':False,'credentials_ingested':False,'mirror_graph_wiring':False},'paths':paths,'schemas':schemas}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: paths={len(paths)}/{len(j.get("paths",{}))} schemas={len(schemas)}/{len(j.get("components",{}).get("schemas",{}))} bytes={out.stat().st_size}')
PY
