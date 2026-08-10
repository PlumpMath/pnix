#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${CELLOSAURUS_IN:-$ROOT/ingest/bio/cellosaurus/cellosaurus.txt}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/cellosaurus-catalog.generated.px}"
LIMIT="${CELLOSAURUS_ENTRY_LIMIT:-1000}"
python3 - "$IN" "$OUT" "$LIMIT" <<'PY'
import re, sys
from pathlib import Path
inp=Path(sys.argv[1]); out=Path(sys.argv[2]); limit=int(sys.argv[3])
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
version=''; last_update=''; total=0; rows=[]; cur=None
def finish(e):
    global total
    if not e or not e.get('accession'): return
    total += 1
    if len(rows) < limit:
        rows.append(e)
for raw in inp.open(encoding='utf-8', errors='replace'):
    line=raw.rstrip('\n')
    if line.startswith(' Version:'):
        version=line.split(':',1)[1].strip()
    elif line.startswith(' Last update:'):
        last_update=line.split(':',1)[1].strip()
    if len(line)<5: continue
    code=line[:2]; val=line[5:].strip() if len(line)>=5 else ''
    if code=='ID':
        finish(cur)
        cur={'id':val,'accession':'','secondary_accessions':[],'synonyms':[],'species':[],'hierarchy_parents':[],'category':'','created':'','last_updated':'','version':''}
    elif cur is None:
        continue
    elif code=='AC':
        cur['accession']=val.rstrip(';')
    elif code=='AS':
        cur['secondary_accessions'] += [x.strip() for x in val.rstrip(';').split(';') if x.strip()]
    elif code=='SY':
        cur['synonyms'] += [x.strip() for x in val.rstrip(';').split(';') if x.strip()][:20]
    elif code=='OX':
        m=re.search(r'NCBI_TaxID=([0-9]+);\s*!\s*(.*)$', val)
        if m:
            cur['species'].append({'tax_id':m.group(1),'label':m.group(2).strip()})
    elif code=='HI':
        m=re.match(r'(CVCL_[A-Za-z0-9]+)\s*!\s*(.*)$', val)
        if m:
            cur['hierarchy_parents'].append({'accession':m.group(1),'label':m.group(2).strip()})
    elif code=='CA':
        cur['category']=val
    elif code=='DT':
        for part in [x.strip() for x in val.split(';') if x.strip()]:
            if part.startswith('Created:'):
                cur['created']=part.split(':',1)[1].strip()
            elif part.startswith('Last updated:'):
                cur['last_updated']=part.split(':',1)[1].strip()
            elif part.startswith('Version:'):
                cur['version']=part.split(':',1)[1].strip()
finish(cur)
seed={'schema':'bio.cellosaurus.catalog.v1','source':{'name':'Cellosaurus','license':'CC-BY-4.0','url':'https://ftp.expasy.org/databases/cellosaurus/cellosaurus.txt','version':version,'last_update':last_update},'summary':{'total_entries_seen':total,'entries_stored':len(rows),'entry_limit':limit,'comments_ingested':False,'profile_data_ingested':False,'disease_rows_ingested':False,'donor_fields_ingested':False,'culture_guidance_ingested':False,'mirror_graph_wiring':False},'entries':rows}
out.write_text(pn(seed)+'\n', encoding='utf-8')
print(f'generated {out}: entries={len(rows)} total_seen={total} bytes={out.stat().st_size}')
PY
