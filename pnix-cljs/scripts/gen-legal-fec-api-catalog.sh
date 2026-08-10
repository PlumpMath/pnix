#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${FEC_API_CATALOG_IN:-$ROOT/ingest/legal/fec-api-catalog/swagger.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/fec-api-catalog.generated.px}"
PATH_LIMIT="${FEC_API_PATH_LIMIT:-160}"
DEF_LIMIT="${FEC_API_DEF_LIMIT:-160}"
python3 - "$IN" "$OUT" "$PATH_LIMIT" "$DEF_LIMIT" <<'PY'
import json, re, sys
from pathlib import Path
inp=Path(sys.argv[1]); out=Path(sys.argv[2]); path_limit=int(sys.argv[3]); def_limit=int(sys.argv[4])
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
allow=re.compile(r'(candidate|committee|filing|form)', re.I)
deny=re.compile(r'(schedule|receipt|disbursement|contribution|contributor|communication|loan|donor|party-coordinated|independent-expenditure)', re.I)
paths=[]; total_candidate=0
for path,ops in sorted(j.get('paths',{}).items()):
    blob=path+' '+json.dumps({k:v.get('tags',[]) if isinstance(v,dict) else [] for k,v in ops.items()}, sort_keys=True)
    if not allow.search(blob) or deny.search(blob):
        continue
    total_candidate += 1
    if len(paths)>=path_limit: continue
    methods=[]
    for method,op in sorted(ops.items()):
        if not isinstance(op,dict): continue
        params=[]
        for p in op.get('parameters',[])[:80]:
            params.append({'name':p.get('name',''),'where':p.get('in',''),'type':p.get('type', p.get('schema',{}).get('type','')),'required':bool(p.get('required',False))})
        responses=[]
        for code,r in sorted(op.get('responses',{}).items())[:20]:
            schema=r.get('schema',{}) if isinstance(r,dict) else {}
            ref=schema.get('$ref','') or schema.get('items',{}).get('$ref','') if isinstance(schema,dict) else ''
            responses.append({'status':str(code),'schema_ref':ref})
        methods.append({'method':method,'operation_id':op.get('operationId',''),'tags':[str(x) for x in op.get('tags',[])[:10]],'parameters':params,'responses':responses})
    paths.append({'path':path,'methods':methods})
defs=[]; total_defs=0
for name,d in sorted(j.get('definitions',{}).items()):
    if not allow.search(name) or deny.search(name):
        continue
    total_defs += 1
    if len(defs)>=def_limit: continue
    props=[]
    for pn_,pv in sorted((d.get('properties') or {}).items())[:120]:
        if deny.search(pn_): continue
        typ=pv.get('type','') if isinstance(pv,dict) else ''
        ref=pv.get('$ref','') if isinstance(pv,dict) else ''
        item_ref=pv.get('items',{}).get('$ref','') if isinstance(pv,dict) and isinstance(pv.get('items'),dict) else ''
        props.append({'name':pn_,'type':typ,'ref':ref or item_ref})
    defs.append({'name':name,'property_count_total':len(d.get('properties') or {}),'properties':props})
seed={'schema':'legal.fec_api_catalog.v1','source':{'name':'FEC OpenFEC API swagger catalog metadata','license':'U.S. government public API documentation metadata','url':'https://api.open.fec.gov/swagger/'},'summary':{'paths_stored':len(paths),'paths_available_candidate_scope':total_candidate,'definitions_stored':len(defs),'definitions_available_candidate_scope':total_defs,'live_api_records_ingested':False,'person_level_rows_ingested':False,'transaction_payloads_ingested':False,'api_keys_ingested':False,'mirror_graph_wiring':False},'paths':paths,'definitions':defs}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: paths={len(paths)}/{total_candidate} defs={len(defs)}/{total_defs} bytes={out.stat().st_size}')
PY
