#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${NIOSH_OSHA_REF_IN:-$ROOT/ingest/tox/niosh-osha-ref-catalog/pages.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/niosh-osha-ref-catalog.generated.px}"
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
    pages.append({'source_id':p.get('source_id',''),'url':p.get('url',''),'http_status':int(p.get('http_status',0)),'title':p.get('title',''),'sha256':p.get('sha256',''),'link_refs':p.get('link_refs',[])[:100]})
seed={'schema':'tox.niosh_osha_ref_catalog.v1','source':{'name':'NIOSH and OSHA occupational chemical reference catalog metadata','license':'U.S. government public information'},'summary':{'page_count':len(pages),'link_ref_count':sum(len(p['link_refs']) for p in pages),'html_bodies_persisted':False,'chemical_entry_bodies_ingested':False,'numeric_exposure_values_ingested':False,'toxicology_prose_ingested':False,'procedure_or_mixing_guidance_ingested':False,'medical_or_legal_advice_ingested':False,'compliance_decisions_ingested':False,'mirror_graph_wiring':False},'pages':pages}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: pages={len(pages)} links={sum(len(p["link_refs"]) for p in pages)} bytes={out.stat().st_size}')
PY
