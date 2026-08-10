#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/bio/ncbi-bioproject-biosample-einfo"
OUT="$ROOT/stdlib/lib/corpus/ncbi-bioproject-biosample-einfo.generated.px"
python3 - "$IN" "$OUT" <<'PY'
import json, sys
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2])
def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
dbs=[]
for p in sorted(root.glob('*.einfo.json')):
    j=json.load(open(p))
    for db in j.get('einforesult',{}).get('dbinfo',[]):
        fields=[]
        for f in db.get('fieldlist',[])[:200]:
            fields.append({k:str(f.get(k,'')) for k in ['name','fullname','termcount','isdate','isnumerical','singletoken','hierarchy','ishidden'] if f.get(k,'') not in (None,'')})
        dbs.append({'dbname':db.get('dbname',''),'menuname':db.get('menuname',''),'description':db.get('description',''),'dbbuild':db.get('dbbuild',''),'count':db.get('count',''),'lastupdate':db.get('lastupdate',''),'field_count_total':len(db.get('fieldlist',[])),'fields':fields})
seed={'schema':'bio.ncbi_bioproject_biosample_einfo.v1','source':{'name':'NCBI EInfo BioProject/BioSample catalog','license':'U.S. government public information','endpoint':'https://eutils.ncbi.nlm.nih.gov/entrez/eutils/einfo.fcgi'},'summary':{'db_count':len(dbs),'record_payloads_ingested':False,'human_genomic_payloads_ingested':False,'sequence_payloads_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'databases':dbs}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: dbs={len(dbs)} fields={sum(len(d["fields"]) for d in dbs)}')
PY
