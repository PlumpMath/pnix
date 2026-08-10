#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${CDC_WONDER_PLACES_IN:-$ROOT/ingest/public_health/cdc-wonder-places-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/cdc-wonder-places-catalog.generated.px}"
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
pages=json.load(open(root/'pages.json')).get('pages',[])
view=json.load(open(root/'places-view.json'))
cols=[]
for c in view.get('columns',[])[:120]:
    cols.append({'name':c.get('name',''),'field_name':c.get('fieldName',''),'data_type_name':c.get('dataTypeName',''),'position':int(c.get('position',0) or 0)})
seed={'schema':'public_health.cdc_wonder_places_catalog.v1','source':{'name':'CDC WONDER and PLACES public catalog metadata','license':'U.S. government public information'},'summary':{'page_count':len(pages),'places_column_count':len(cols),'observation_rows_ingested':False,'geography_time_measure_values_ingested':False,'small_cell_data_ingested':False,'person_level_records_ingested':False,'health_advice_ingested':False,'epidemiological_interpretation_ingested':False,'mirror_graph_wiring':False},'pages':[{'source_id':p.get('source_id',''),'url':p.get('url',''),'http_status':int(p.get('http_status',0)),'title':p.get('title',''),'sha256':p.get('sha256',''),'link_refs':p.get('link_refs',[])[:80]} for p in pages],'places_view':{'id':view.get('id',''),'name':view.get('name',''),'license':(view.get('license') or {}).get('name','') if isinstance(view.get('license'),dict) else str(view.get('license','')),'rows_updated_at':str(view.get('rowsUpdatedAt','')),'columns':cols}}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: pages={len(pages)} columns={len(cols)} bytes={out.stat().st_size}')
PY
