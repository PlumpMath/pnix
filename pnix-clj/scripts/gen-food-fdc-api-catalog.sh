#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${FDC_API_CATALOG_IN:-$ROOT/ingest/food/fdc-api-catalog/pages.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/fdc-api-catalog.generated.px}"
python3 - "$IN" "$OUT" <<'PY'
import json, sys
from pathlib import Path
inp=Path(sys.argv[1]); out=Path(sys.argv[2])
def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
j=json.load(open(inp)); page=j.get('page',{})
seed={'schema':'food.fdc_api_catalog.v1','source':{'name':'USDA FoodData Central API guide catalog metadata','license':'USDA public API guide metadata'},'summary':{'api_path_ref_count':len(page.get('api_path_refs',[])),'html_body_persisted':False,'food_item_rows_ingested':False,'nutrient_values_ingested':False,'branded_product_payloads_ingested':False,'ingredients_or_allergens_ingested':False,'api_result_rows_ingested':False,'dietary_or_medical_advice_ingested':False,'mirror_graph_wiring':False},'page':{'url':page.get('url',''),'http_status':int(page.get('http_status',0)),'title':page.get('title',''),'sha256':page.get('sha256',''),'api_path_refs':page.get('api_path_refs',[])[:80]}}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: refs={len(seed["page"]["api_path_refs"])} bytes={out.stat().st_size}')
PY
