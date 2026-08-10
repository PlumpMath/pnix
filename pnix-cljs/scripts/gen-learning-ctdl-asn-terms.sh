#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/learning/ctdl-asn-terms"
OUT="$ROOT/stdlib/lib/corpus/ctdl-asn-terms.generated.px"
TERM_LIMIT="${CTDL_ASN_TERM_LIMIT:-1200}"
TRIPLE_LIMIT="${CTDL_ASN_TRIPLE_LIMIT:-3000}"
python3 - "$SRC" "$OUT" "$TERM_LIMIT" "$TRIPLE_LIMIT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); term_limit=int(sys.argv[3]); triple_limit=int(sys.argv[4])
manifest=json.loads((src/'source-manifest.json').read_text()) if (src/'source-manifest.json').exists() else {}
text=(src/'raw'/'ctdl-asn-terms.ttl').read_text(encoding='utf-8',errors='replace')
def lit(x):
    if x is None: return 'null'
    if x is True: return 'true'
    if x is False: return 'false'
    if isinstance(x,int): return str(x)
    if isinstance(x,float): return repr(x)
    if isinstance(x,str): return json.dumps(x,ensure_ascii=False)
    if isinstance(x,list): return '[ ' + ' '.join(lit(v) for v in x) + ' ]'
    if isinstance(x,dict): return '{ ' + ' '.join(json.dumps(str(k),ensure_ascii=False)+' = '+lit(v)+';' for k,v in sorted(x.items())) + ' }'
    raise TypeError(type(x))
def strip_comments(t):
    out=[]
    for line in t.splitlines():
        buf=[]; ins=False; esc=False; q=''
        for c in line:
            if ins:
                buf.append(c)
                if esc: esc=False
                elif c=='\\': esc=True
                elif c==q: ins=False
            else:
                if c in ('"',"'"):
                    ins=True; q=c; buf.append(c)
                elif c=='#': break
                else: buf.append(c)
        out.append(''.join(buf))
    return '\n'.join(out)
def split_top(s, sep):
    res=[]; buf=[]; depth=0; ins=False; esc=False; q=''
    for c in s:
        if ins:
            buf.append(c)
            if esc: esc=False
            elif c=='\\': esc=True
            elif c==q: ins=False
        else:
            if c in ('"',"'"):
                ins=True; q=c; buf.append(c)
            elif c in '([{': depth+=1; buf.append(c)
            elif c in ')]}': depth=max(0,depth-1); buf.append(c)
            elif c==sep and depth==0:
                part=''.join(buf).strip()
                if part: res.append(part)
                buf=[]
            else: buf.append(c)
    part=''.join(buf).strip()
    if part: res.append(part)
    return res
def statements(t):
    t=strip_comments(t); buf=[]; depth=0; ins=False; esc=False; q=''
    for c in t:
        if ins:
            buf.append(c)
            if esc: esc=False
            elif c=='\\': esc=True
            elif c==q: ins=False
        else:
            if c in ('"',"'"):
                ins=True; q=c; buf.append(c)
            elif c in '([{': depth+=1; buf.append(c)
            elif c in ')]}': depth=max(0,depth-1); buf.append(c)
            elif c=='.' and depth==0:
                st=''.join(buf).strip()
                if st: yield st
                buf=[]
            else: buf.append(c)
prefixes=[{'prefix':m.group(1),'iri':m.group(2)} for m in re.finditer(r'@prefix\s+([^:\s]+):\s*<([^>]+)>\s*\.', text)]
prose_words=('comment','definition','description','note','example','history','usage','scopeNote','editorialNote')
keep={'a','rdf:type','rdfs:label','rdfs:domain','rdfs:range','rdfs:subClassOf','rdfs:subPropertyOf','owl:inverseOf','owl:equivalentClass','owl:equivalentProperty','skos:inScheme','vs:term_status','schema:domainIncludes','schema:rangeIncludes'}
triples=[]; terms={}; skipped=0; raw=0
for st in statements(text):
    if st.startswith('@prefix') or st.startswith('@base'): continue
    parts=split_top(st,';')
    if not parts: continue
    first=parts[0].split(None,2)
    if len(first)<3: continue
    subj,pred,objstr=first
    pred_objs=[(pred,objstr)]
    for part in parts[1:]:
        po=part.split(None,1)
        if len(po)==2: pred_objs.append((po[0],po[1]))
    for pred,objstr in pred_objs:
        if any(w.lower() in pred.lower() for w in prose_words):
            skipped+=1; continue
        for obj in split_top(objstr,','):
            obj=obj.strip()
            if not obj: continue
            raw+=1
            if len(triples)<triple_limit: triples.append({'subject':subj,'predicate':pred,'object':obj})
            if pred in keep:
                t=terms.setdefault(subj,{'id':subj,'props':{}})
                vals=t['props'].setdefault(pred,[])
                if len(vals)<12: vals.append(obj)
term_rows=[]
for _,t in sorted(terms.items()):
    if len(term_rows)>=term_limit: break
    term_rows.append(t)
obj={'schema':'learning.ctdl_asn.terms.v1','source':'CTDL-ASN schema terms structural metadata','license':'CC-BY-4.0','policy':'Turtle structural terms only; comments/definitions/examples/guidance/registry records/graph wiring excluded','summary':{'prefixes':len(prefixes),'terms':len(term_rows),'triples':len(triples),'raw_triples_seen':raw,'prose_predicates_skipped':skipped},'limits':{'term_limit':term_limit,'triple_limit':triple_limit},'manifest':manifest,'prefixes':prefixes,'terms':term_rows,'triples':triples}
out.write_text('# GENERATED by scripts/gen-learning-ctdl-asn-terms.sh. Do not edit. Gitignored.\n# Source: CTDL-ASN Turtle structural metadata only.\n'+lit(obj)+'\n',encoding='utf-8')
print(f'generated {out}: terms={len(term_rows)} triples={len(triples)}/{raw} skipped={skipped} bytes={out.stat().st_size}')
PY
