#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/food/openfda-food-enforcement/food-enforcement.json"
OUT="$ROOT/stdlib/lib/corpus/openfda-food-enforcement.generated.px"
python3 - "$IN" "$OUT" <<'PY'
import json, sys
from pathlib import Path
j=json.load(open(sys.argv[1])); out=Path(sys.argv[2])

def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
fields=['event_id','recall_number','status','classification','product_type','state','country','voluntary_mandated','initial_firm_notification','recall_initiation_date','center_classification_date','termination_date','report_date']
rows=[]
for r in j.get('results',[]):
    row={k:str(r.get(k,'')) for k in fields if r.get(k,'') not in (None,'')}
    rows.append(row)
meta=j.get('meta',{})
seed={'schema':'food.openfda_food_enforcement.v1','source':{'name':'openFDA Food Enforcement','license':'CC0-1.0 / FDA public domain unless otherwise noted','endpoint':'https://api.fda.gov/food/enforcement.json'},'summary':{'last_updated':meta.get('last_updated',''),'api_total':meta.get('results',{}).get('total',0),'stored_count':len(rows),'narrative_prose_ingested':False,'product_details_ingested':False,'address_firm_details_ingested':False,'food_medical_advice_ingested':False,'mirror_graph_wiring':False},'enforcements':rows}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: records={len(rows)} total={meta.get("results",{}).get("total",0)}')
PY
