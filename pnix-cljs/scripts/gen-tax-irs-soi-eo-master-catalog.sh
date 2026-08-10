#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${IRS_SOI_EO_MASTER_IN:-$ROOT/ingest/tax/irs-soi-eo-master-catalog/pages.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/irs-soi-eo-master-catalog.generated.px}"
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
    pages.append({'source_id':p.get('source_id',''),'url':p.get('url',''),'http_status':int(p.get('http_status',0)),'title':p.get('title',''),'sha256':p.get('sha256',''),'download_file_refs':p.get('download_file_refs',[])[:120]})
seed={'schema':'tax.irs_soi_eo_master_catalog.v1','source':{'name':'IRS SOI and EO master public catalog metadata','license':'U.S. government public information'},'summary':{'page_count':len(pages),'download_file_ref_count':sum(len(p['download_file_refs']) for p in pages),'html_bodies_persisted':False,'download_payloads_ingested':False,'return_payloads_ingested':False,'search_result_rows_ingested':False,'organization_record_values_ingested':False,'advice_or_eligibility_ingested':False,'mirror_graph_wiring':False},'pages':pages}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: pages={len(pages)} file_refs={sum(len(p["download_file_refs"]) for p in pages)} bytes={out.stat().st_size}')
PY
