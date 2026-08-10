#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/culture/smithsonian-openaccess/search.json"
OUT="$ROOT/stdlib/lib/corpus/smithsonian-openaccess.generated.px"
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
rows=[]
for r in j.get('response',{}).get('rows',[]):
    row={}
    for k in ['id','title','unitCode','type','url']:
        if r.get(k) not in (None,''): row[k]=r.get(k)
    if row: rows.append(row)
seed={'schema':'culture.smithsonian_openaccess.v1','source':{'name':'Smithsonian Open Access API','license':'Smithsonian Open Access / CC0-style public metadata','endpoint':'https://api.si.edu/openaccess/api/v1.0/search'},'summary':{'status':j.get('status',0),'responseCode':j.get('responseCode',0),'stored_count':len(rows),'freetext_payloads_ingested':False,'media_payloads_ingested':False,'person_location_details_ingested':False,'rights_interpretation_ingested':False,'mirror_graph_wiring':False},'objects':rows}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: objects={len(rows)}')
PY
