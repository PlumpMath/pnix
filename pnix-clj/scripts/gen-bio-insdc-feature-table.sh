#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/bio/insdc-feature-table/feature-table.html"
OUT="$ROOT/stdlib/lib/corpus/insdc-feature-table.generated.px"
python3 - "$IN" "$OUT" <<'PY'
import html, re, sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2])
s=html.unescape(src.read_text(errors='ignore'))
# Strip tags but preserve line boundaries from <pre> content.
s=re.sub(r'<[^>]+>','',s)

def esc(x): return '"'+x.replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
features=[]
blocks=re.split(r'\nFeature Key\s+', s)
for b in blocks[1:]:
    lines=b.splitlines()
    if not lines: continue
    key=lines[0].strip().split()[0]
    if not re.match(r'^[A-Za-z0-9_\-]+$', key): continue
    mandatory=[]; optional=[]; mode=None
    for line in lines[1:]:
        if line.startswith('Feature Key') or line.startswith('7.3'): break
        if line.startswith('Mandatory qualifiers'): mode='mandatory'; tail=line[len('Mandatory qualifiers'):]
        elif line.startswith('Optional qualifiers'): mode='optional'; tail=line[len('Optional qualifiers'):]
        elif re.match(r'^[A-Z][A-Za-z ]{3,}\s{2,}', line): mode=None; tail=''
        else: tail=line if mode else ''
        if mode and tail:
            for q in re.findall(r'/([A-Za-z0-9_]+)\b', tail):
                (mandatory if mode=='mandatory' else optional).append(q)
    features.append({'key':key,'mandatory_qualifiers':sorted(set(mandatory)),'optional_qualifiers':sorted(set(optional))})
qualifiers=[]
q_start=s.rfind('7.3.1 Qualifier List')
q_end=s.rfind('7.4 Appendix IV')
qsec=s[q_start:(q_end if q_end>q_start else len(s))]
for m in re.finditer(r'\nQualifier\s+/([A-Za-z0-9_]+)(=?)', qsec):
    qualifiers.append({'key':m.group(1),'requires_value':m.group(2)=='='})
# controlled vocabulary code rows: parse compact listing sections by fixed-width first token only, bounded.
controlled=[]
for sec_id, sec_name in [('7.4.1','nucleotide_base_codes'),('7.4.2','modified_base_abbreviations'),('7.4.3','amino_acid_abbreviations'),('7.4.4','modified_unusual_amino_acids')]:
    idx=s.rfind(sec_id)
    if idx<0: continue
    nxt=s.find('7.4.', idx+len(sec_id))
    chunk=s[idx:nxt if nxt>idx else idx+8000]
    for line in chunk.splitlines():
        line=line.strip()
        if not line or line.startswith(('Authority','Reference','Contact','Scope','Listing','Symbol','------','Abbreviation','Amino')): continue
        parts=line.split()
        if parts and re.match(r'^[A-Za-z0-9][A-Za-z0-9\-]*$', parts[0]) and len(parts[0]) <= 20:
            token=parts[0]
            if token.lower() not in {'and','the','note','modified','amino','scope','contact','reference'}:
                controlled.append({'vocabulary':sec_name,'code':token})
controlled=controlled[:500]
seed={'schema':'bio.insdc_feature_table.v1','source':{'name':'DDBJ/ENA/GenBank Feature Table Definition','url':'https://www.insdc.org/submitting-standards/feature-table/','license':'INSDC public submission standard / public vocabulary'},'summary':{'feature_key_count':len(features),'qualifier_key_count':len(qualifiers),'controlled_code_count':len(controlled),'definitions_ingested':False,'comments_ingested':False,'examples_ingested':False,'sequence_payloads_ingested':False,'mirror_graph_wiring':False},'feature_keys':features,'qualifier_keys':sorted(qualifiers,key=lambda x:x['key']),'controlled_codes':controlled}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: features={len(features)} qualifiers={len(qualifiers)} controlled={len(controlled)}')
PY
