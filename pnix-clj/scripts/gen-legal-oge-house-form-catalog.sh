#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${OGE_HOUSE_FORM_IN:-$ROOT/ingest/legal/oge-house-form-catalog/pages.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/oge-house-form-catalog.generated.px}"
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
j=json.load(open(inp))
pages=[]
for p in j.get('pages',[]):
    pages.append({'source_id':p.get('source_id',''),'url':p.get('url',''),'http_status':int(p.get('http_status',0)),'title':p.get('title',''),'sha256':p.get('sha256',''),'form_link_refs':p.get('form_link_refs',[])[:80]})
seed={'schema':'legal.oge_house_form_catalog.v1','source':{'name':'OGE and House ethics form catalog metadata','license':'U.S. government public information'},'summary':{'page_count':len(pages),'form_link_ref_count':sum(len(p['form_link_refs']) for p in pages),'html_bodies_persisted':False,'form_bodies_ingested':False,'submitted_report_records_ingested':False,'person_or_filer_data_ingested':False,'legal_or_ethics_advice_ingested':False,'mirror_graph_wiring':False},'pages':pages}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: pages={len(pages)} form_refs={sum(len(p["form_link_refs"]) for p in pages)} bytes={out.stat().st_size}')
PY
