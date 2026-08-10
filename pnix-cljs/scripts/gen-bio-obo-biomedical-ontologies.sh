#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/bio/obo-biomedical-ontologies"
OUT="$ROOT/stdlib/lib/corpus/obo-biomedical-ontologies.generated.px"
LIMIT="${OBO_BIOMED_TERM_LIMIT:-450}"
python3 - "$IN" "$OUT" "$LIMIT" <<'PY'
import sys
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2]); limit=int(sys.argv[3])
SOURCES=[('doid','Disease Ontology','CC0-1.0'),('hp','Human Phenotype Ontology','CC-BY-4.0'),('so','Sequence Ontology','CC-BY-4.0')]

def esc(s): return '"'+s.replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))

def parse_obo(path, source_id):
    total=0; terms=[]; cur=None
    def flush():
        nonlocal total, cur
        if cur and not cur.get('obsolete'):
            total += 1
            if len(terms) < limit:
                terms.append({k:v for k,v in cur.items() if v not in (None,'',[])})
        cur=None
    for raw in path.read_text(errors='replace').splitlines():
        line=raw.strip()
        if not line: continue
        if line=='[Term]': flush(); cur={'source_id':source_id,'id':'','name':'','namespace':'','is_a':[],'relationships':[]}; continue
        if line.startswith('['): flush(); cur=None; continue
        if cur is None or ': ' not in line: continue
        key,val=line.split(': ',1)
        val=val.split(' ! ',1)[0].strip()
        if key=='id': cur['id']=val
        elif key=='name': cur['name']=val
        elif key=='namespace': cur['namespace']=val
        elif key=='is_a': cur['is_a'].append(val.split()[0])
        elif key=='relationship':
            parts=val.split()
            if len(parts)>=2: cur['relationships'].append({'predicate':parts[0], 'target':parts[1]})
        elif key=='is_obsolete' and val.lower()=='true': cur['obsolete']=True
    flush()
    return total, terms
summaries=[]; all_terms=[]
for sid,name,lic in SOURCES:
    total, terms=parse_obo(root/f'{sid}.obo', sid)
    summaries.append({'source_id':sid,'name':name,'license':lic,'term_count_total':total,'term_count_stored':len(terms)})
    all_terms += terms
seed={'schema':'bio.obo_biomedical_ontologies.v1','source':{'name':'OBO biomedical ontology term structure','licenses':'Disease Ontology CC0; HPO and Sequence Ontology CC-BY-4.0','source_ids':[s[0] for s in SOURCES]},'summary':{'source_count':len(SOURCES),'term_count_stored':len(all_terms),'definition_prose_ingested':False,'synonym_prose_ingested':False,'patient_payloads_ingested':False,'sequence_payloads_ingested':False,'mirror_graph_wiring':False,'sources':summaries},'terms':all_terms}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: terms={len(all_terms)} summaries={summaries}')
PY
