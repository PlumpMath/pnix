#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/code/oslc-rm-vocab/raw"
OUT="$ROOT/stdlib/lib/corpus/oslc-rm-vocab.generated.px"
RECEIPT="$ROOT/ingest/code/oslc-rm-vocab/source-receipt.json"
if [[ ! -d "$SRC" ]]; then echo "missing $SRC; run update first" >&2; exit 1; fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, re, sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt=json.loads(Path(sys.argv[3]).read_text())
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)

def strip_comments(text):
    return '\n'.join(line.split('#',1)[0] for line in text.splitlines())
def split_top(s, sep):
    res=[]; buf=[]; depth=0; ins=False; escp=False; quote=''
    for c in s:
        if ins:
            buf.append(c)
            if escp: escp=False
            elif c=='\\': escp=True
            elif c==quote: ins=False
        else:
            if c in ('"',"'"):
                ins=True; quote=c; buf.append(c)
            elif c in '([{': depth+=1; buf.append(c)
            elif c in ')]}': depth=max(0,depth-1); buf.append(c)
            elif c==sep and depth==0:
                p=''.join(buf).strip()
                if p: res.append(p)
                buf=[]
            else: buf.append(c)
    p=''.join(buf).strip()
    if p: res.append(p)
    return res
def statements(text):
    text=strip_comments(text); buf=[]; depth=0; ins=False; escp=False; quote=''
    for c in text:
        if ins:
            buf.append(c)
            if escp: escp=False
            elif c=='\\': escp=True
            elif c==quote: ins=False
        else:
            if c in ('"',"'"):
                ins=True; quote=c; buf.append(c)
            elif c in '([{': depth+=1; buf.append(c)
            elif c in ')]}': depth=max(0,depth-1); buf.append(c)
            elif c=='.' and depth==0:
                st=''.join(buf).strip()
                if st: yield st
                buf=[]
            else: buf.append(c)
files=[]; prefixes=[]; triples=[]; terms={}; skipped=0
prose=('comment','description','definition','example','abstract')
keep={'a','rdf:type','rdfs:label','rdfs:isDefinedBy','rdfs:range','rdfs:domain','rdfs:subClassOf','rdfs:subPropertyOf','oslc:propertyDefinition','oslc:occurs','oslc:valueType','oslc:range','oslc:representation','oslc:readOnly','oslc:name','dcterms:source','dcterms:isPartOf','dcterms:license'}
for path in sorted(src.glob('*.ttl')):
    text=path.read_text(errors='replace')
    files.append({'file':path.name,'bytes':path.stat().st_size,'lines':len(text.splitlines())})
    for m in re.finditer(r'@prefix\s+([^:\s]+):\s*<([^>]+)>\s*\.', text):
        prefixes.append({'file':path.name,'prefix':m.group(1),'iri':m.group(2)})
    for st in statements(text):
        if st.startswith('@prefix') or st.startswith('@base'): continue
        parts=split_top(st,';')
        if not parts: continue
        first=parts[0].split(None,2)
        if len(first)<3: continue
        subj,pred,objstr=first
        pos=[(pred,objstr)]
        for part in parts[1:]:
            xs=part.split(None,1)
            if len(xs)==2: pos.append((xs[0],xs[1]))
        for pred,objstr in pos:
            if any(x in pred.lower() for x in prose):
                skipped+=1; continue
            for obj in split_top(objstr, ','):
                obj=obj.strip()
                if not obj: continue
                triples.append({'file':path.name,'subject':subj,'predicate':pred,'object':obj})
                if pred in keep:
                    t=terms.setdefault(subj, {'id':subj,'file':path.name,'props':{}})
                    vals=t['props'].setdefault(pred, [])
                    if len(vals)<8: vals.append(obj)
terms_list=[{'id':t['id'],'file':t['file'],'props':t['props']} for _,t in sorted(terms.items())]
data={'schema':'code.oslc_rm.vocab.v1','source':'OSLC RM 2.1 vocabulary/shapes TTL structural metadata','license':'Apache-2.0','ref':receipt.get('ref','master'),'source_receipt':receipt,'source_files':files,'prefixes':prefixes,'triples':triples,'terms':terms_list,'summary':{'triple_count':len(triples),'term_count':len(terms_list),'prose_predicates_skipped':skipped},'exclusions':['comments/descriptions','spec prose','examples','live requirements documents','OSLC service data','credentials','execution','mirror/graph wiring']}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: files={len(files)} triples={len(triples)} terms={len(terms_list)} skipped_prose={skipped} bytes={out.stat().st_size}")
PY
