#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${OQMD_OPTIMADE_IN:-$ROOT/ingest/materials/oqmd-optimade-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/oqmd-optimade-catalog.generated.px}"
python3 - "$IN" "$OUT" <<'PY'
import json, sys
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2])
def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
info=json.load(open(root/'info.json'))
links=json.load(open(root/'links.json'))
meta=info.get('meta',{})
prov=meta.get('provider',{}) if isinstance(meta.get('provider',{}),dict) else {}
link_rows=[]
for x in links.get('data',[])[:50]:
    a=x.get('attributes',{}) if isinstance(x,dict) else {}
    link_rows.append({'id':x.get('id',''),'type':x.get('type',''),'link_type':a.get('link_type',''),'base_url':a.get('base_url',''),'prefix':a.get('prefix','')})
seed={'schema':'materials.oqmd_optimade_catalog.v1','source':{'name':'OQMD OPTIMADE provider catalog metadata','license':'public endpoint metadata','base':'https://oqmd.org/optimade/v1'},'summary':{'link_count':len(link_rows),'structure_rows_ingested':False,'composition_values_ingested':False,'property_values_ingested':False,'calculation_payloads_ingested':False,'descriptions_or_prose_ingested':False,'query_execution':False,'mirror_graph_wiring':False},'provider':{'api_version':meta.get('api_version',''),'schema_url':meta.get('schema',''),'provider_name':prov.get('name',''),'provider_prefix':prov.get('prefix','')},'links':link_rows}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: links={len(link_rows)} bytes={out.stat().st_size}')
PY
