#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/culture/met-open-access"
OUT="$ROOT/stdlib/lib/corpus/met-open-access.generated.px"
LIMIT="${MET_OBJECT_LIMIT:-60}"
python3 - "$IN" "$OUT" "$LIMIT" <<'PY'
import json, sys
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2]); limit=int(sys.argv[3])

def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
deps=json.load(open(root/'departments.json')).get('departments',[])
deps=[{'departmentId':d.get('departmentId',0),'displayName':d.get('displayName','')} for d in deps]
objs=[]
for p in sorted((root/'objects').glob('*.json')):
    if len(objs)>=limit: break
    try: j=json.load(open(p))
    except Exception: continue
    if j.get('isPublicDomain') is not True: continue
    try:
        if int(j.get('objectEndDate') or 9999) > 1925: continue
    except Exception: continue
    row={}
    for k in ['objectID','department','objectName','classification','culture','period','medium','objectBeginDate','objectEndDate','objectURL']:
        v=j.get(k,'')
        if v not in (None,''): row[k]=v
    row['isPublicDomain']=True
    objs.append(row)
seed={'schema':'culture.met_open_access.v1','source':{'name':'The Met Open Access Collection API','license':'CC0-1.0','api':'https://collectionapi.metmuseum.org/public/collection/v1/'},'summary':{'department_count':len(deps),'object_count_stored':len(objs),'public_domain_filter':True,'end_date_lte':1925,'images_ingested':False,'image_urls_ingested':False,'provenance_prose_ingested':False,'artist_person_metadata_ingested':False,'mirror_graph_wiring':False},'departments':deps,'objects':objs}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: departments={len(deps)} objects={len(objs)}')
PY
