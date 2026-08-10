#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${OPTIMADE_SCHEMA_IN:-$ROOT/ingest/materials/optimade-schema-catalog/tree.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/optimade-schema-catalog.generated.px}"
LIMIT="${OPTIMADE_SCHEMA_FILE_LIMIT:-500}"
python3 - "$IN" "$OUT" "$LIMIT" <<'PY'
import json, sys
from pathlib import Path
inp=Path(sys.argv[1]); out=Path(sys.argv[2]); limit=int(sys.argv[3])
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
files=[]; dirs=0; cats={}
for x in j.get('tree',[]):
    typ=x.get('type',''); path=x.get('path','')
    if typ=='tree':
        dirs += 1; continue
    if typ!='blob': continue
    top='/'.join(path.split('/')[:4]) if '/' in path else path
    cats[top]=cats.get(top,0)+1
    if len(files)<limit:
        files.append({'path':path,'size':int(x.get('size') or 0),'sha':x.get('sha',''),'category':top})
seed={'schema':'materials.optimade_schema_catalog.v1','source':{'name':'OPTIMADE specification schema tree metadata','license':'CC-BY-4.0','repository':'Materials-Consortia/OPTIMADE','ref':j.get('ref',''),'schemas_src_sha':j.get('schemas_src_sha','')},'summary':{'files_stored':len(files),'files_available':sum(1 for x in j.get('tree',[]) if x.get('type')=='blob'),'directories_available':dirs,'truncated':bool(j.get('truncated',False)),'schema_file_limit':limit,'provider_rows_ingested':False,'material_structure_payloads_ingested':False,'composition_values_ingested':False,'computed_properties_ingested':False,'account_or_embargoed_rows_ingested':False,'schema_bodies_ingested':False,'mirror_graph_wiring':False},'category_counts':[{'category':k,'count':v} for k,v in sorted(cats.items())],'files':files}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: files={len(files)}/{seed["summary"]["files_available"]} bytes={out.stat().st_size}')
PY
