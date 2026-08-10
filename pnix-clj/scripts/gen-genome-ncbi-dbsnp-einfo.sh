#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${NCBI_DBSNP_EINFO_IN:-$ROOT/ingest/genome/ncbi-dbsnp-einfo}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/ncbi-dbsnp-einfo.generated.px}"
python3 - "$IN" "$OUT" <<'PY'
import json, sys
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2])
def esc(s):
    return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
p=root/'snp.einfo.json'
j=json.load(open(p))
dbs=[]
for db in j.get('einforesult',{}).get('dbinfo',[]):
    fields=[]
    for f in db.get('fieldlist',[])[:250]:
        fields.append({
            'name': str(f.get('name','')),
            'fullname': str(f.get('fullname','')),
            'termcount': str(f.get('termcount','')),
            'isdate': str(f.get('isdate','')),
            'isnumerical': str(f.get('isnumerical','')),
            'singletoken': str(f.get('singletoken','')),
            'hierarchy': str(f.get('hierarchy','')),
            'ishidden': str(f.get('ishidden','')),
        })
    dbs.append({
        'dbname': str(db.get('dbname','')),
        'menuname': str(db.get('menuname','')),
        'description': str(db.get('description','')),
        'dbbuild': str(db.get('dbbuild','')),
        'count': str(db.get('count','')),
        'lastupdate': str(db.get('lastupdate','')),
        'field_count_total': len(db.get('fieldlist',[])),
        'fields': fields,
    })
seed={
    'schema':'genome.ncbi_dbsnp_einfo.v1',
    'source':{
        'name':'NCBI dbSNP EInfo catalog metadata',
        'license':'U.S. government public information',
        'endpoint':'https://eutils.ncbi.nlm.nih.gov/entrez/eutils/einfo.fcgi?db=snp&retmode=json',
    },
    'summary':{
        'db_count':len(dbs),
        'variant_record_payloads_ingested':False,
        'genotype_sample_payloads_ingested':False,
        'sequence_payloads_ingested':False,
        'linked_payloads_ingested':False,
        'medical_interpretation_ingested':False,
        'mirror_graph_wiring':False,
    },
    'databases':dbs,
}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: dbs={len(dbs)} fields={sum(len(d["fields"]) for d in dbs)} bytes={out.stat().st_size}')
PY
