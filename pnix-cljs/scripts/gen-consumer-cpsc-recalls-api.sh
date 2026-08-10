#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/consumer/cpsc-recalls-api/recalls.json"
OUT="$ROOT/stdlib/lib/corpus/cpsc-recalls-api.generated.px"
LIMIT="${CPSC_RECALL_LIMIT:-500}"
python3 - "$IN" "$OUT" "$LIMIT" <<'PY'
import json, sys
from pathlib import Path
rows=json.load(open(sys.argv[1])); out=Path(sys.argv[2]); limit=int(sys.argv[3])

def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
def toks(items, fields, maxn=8):
    out=[]
    for it in items or []:
        d={}
        for f in fields:
            v=it.get(f,'') if isinstance(it,dict) else ''
            if v not in (None,''): d[f]=str(v)
        if d: out.append(d)
        if len(out)>=maxn: break
    return out
recs=[]
for r in rows[:limit]:
    rec={}
    for k in ['RecallID','RecallNumber','RecallDate','URL','LastPublishDate']:
        if r.get(k) not in (None,''): rec[k]=r.get(k)
    rec['products']=toks(r.get('Products'), ['CategoryID','Type'], 8)
    rec['hazards']=toks(r.get('Hazards'), ['HazardTypeID','HazardType'], 8)
    rec['remedy_options']=toks(r.get('RemedyOptions'), ['Option'], 8)
    rec['manufacturer_countries']=toks(r.get('ManufacturerCountries'), ['Country'], 8)
    recs.append(rec)
seed={'schema':'consumer.cpsc_recalls_api.v1','source':{'name':'U.S. CPSC Recalls API','license':'U.S. federal public information','endpoint':'https://www.saferproducts.gov/RestWebServices/Recall?format=json'},'summary':{'api_total':len(rows),'stored_count':len(recs),'narrative_prose_ingested':False,'image_payloads_ingested':False,'contact_text_ingested':False,'company_identifiers_ingested':False,'legal_safety_advice_ingested':False,'mirror_graph_wiring':False},'recalls':recs}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: recalls={len(recs)}/{len(rows)}')
PY
