#!/usr/bin/env bash
set -euo pipefail

# Convert QUDT RDF/Turtle schema/vocab files into bounded pnix attrset source.
# Host code transcribes RDF tokens only. It does not run unit conversion or wire graph/mirror logic.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/science/qudt-ontology/raw"
OUT="$ROOT/stdlib/lib/corpus/qudt-ontology.generated.px"
RECEIPT="$ROOT/ingest/science/qudt-ontology/source-receipt.json"
TRIPLE_LIMIT="${QUDT_TRIPLE_LIMIT:-3500}"
TERM_LIMIT="${QUDT_TERM_LIMIT:-1800}"
if [[ ! -d "$SRC" ]]; then
  echo "missing $SRC; run scripts/update-science-qudt-ontology.sh first" >&2
  exit 1
fi

python3 - "$SRC" "$OUT" "$RECEIPT" "$TRIPLE_LIMIT" "$TERM_LIMIT" <<'PY'
import json, re, sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
triple_limit=int(sys.argv[4]); term_limit=int(sys.argv[5])
receipt=json.loads(receipt_path.read_text()) if receipt_path.exists() else {}

def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return "true" if v else "false"
    if isinstance(v,int): return str(v)
    if isinstance(v,float): return esc(v)
    if isinstance(v,list): return "[ " + " ".join(to_pnix(x) for x in v) + " ]"
    if isinstance(v,dict): return "{ " + " ".join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + " }"
    return esc(v)

def strip_comments(text):
    out=[]; in_s=False; escp=False; quote=''
    for line in text.splitlines():
        buf=[]; in_s=False; escp=False; quote=''
        i=0
        while i < len(line):
            c=line[i]
            if in_s:
                buf.append(c)
                if escp: escp=False
                elif c=='\\': escp=True
                elif c==quote: in_s=False
            else:
                if c in ('"', "'"):
                    in_s=True; quote=c; buf.append(c)
                elif c=='#':
                    break
                else:
                    buf.append(c)
            i+=1
        out.append(''.join(buf))
    return '\n'.join(out)

def split_top(s, sep):
    res=[]; buf=[]; depth=0; in_s=False; escp=False; quote=''
    for c in s:
        if in_s:
            buf.append(c)
            if escp: escp=False
            elif c=='\\': escp=True
            elif c==quote: in_s=False
        else:
            if c in ('"', "'"):
                in_s=True; quote=c; buf.append(c)
            elif c in '([{':
                depth+=1; buf.append(c)
            elif c in ')]}':
                depth=max(0,depth-1); buf.append(c)
            elif c==sep and depth==0:
                part=''.join(buf).strip()
                if part: res.append(part)
                buf=[]
            else:
                buf.append(c)
    part=''.join(buf).strip()
    if part: res.append(part)
    return res

def statements(text):
    text=strip_comments(text)
    buf=[]; depth=0; in_s=False; escp=False; quote=''
    for c in text:
        if in_s:
            buf.append(c)
            if escp: escp=False
            elif c=='\\': escp=True
            elif c==quote: in_s=False
        else:
            if c in ('"', "'"):
                in_s=True; quote=c; buf.append(c)
            elif c in '([{':
                depth+=1; buf.append(c)
            elif c in ')]}':
                depth=max(0,depth-1); buf.append(c)
            elif c=='.' and depth==0:
                st=''.join(buf).strip()
                if st: yield st
                buf=[]
            else:
                buf.append(c)

prose_words=('description','comment','definition','example','citation','note','abstract','editorial','history','todo')
keep_props={
 'a','rdf:type','rdfs:label','skos:prefLabel','qudt:symbol','qudt:ucumCode','qudt:uneceCommonCode','qudt:conversionMultiplier','qudt:conversionOffset',
 'qudt:hasDimensionVector','qudt:hasQuantityKind','qudt:applicableUnit','qudt:prefixMultiplier','qudt:prefixMultiplierSN','qudt:quantityKind',
 'qudt:hasUnit','qudt:systemUnit','qudt:hasBaseUnit','qudt:baseUnitDimensions','qudt:dimensionExponent','qudt:dimensionExponentForAmountOfSubstance',
 'qudt:dimensionExponentForElectricCurrent','qudt:dimensionExponentForLength','qudt:dimensionExponentForLuminousIntensity','qudt:dimensionExponentForMass',
 'qudt:dimensionExponentForThermodynamicTemperature','qudt:dimensionExponentForTime','skos:broader','skos:narrower','rdfs:subClassOf','rdfs:subPropertyOf'
}
source_files=[]; prefixes=[]; triples=[]; terms={}; skipped_prose=0; raw_triples=0
for path in sorted(src.glob('*.ttl')):
    rel=path.name.replace('__','/')
    text=path.read_text(errors='replace')
    source_files.append({'path':rel,'bytes':path.stat().st_size,'lines':len(text.splitlines())})
    for m in re.finditer(r'@prefix\s+([^:\s]+):\s*<([^>]+)>\s*\.', text):
        prefixes.append({'file':rel,'prefix':m.group(1),'iri':m.group(2)})
    for st in statements(text):
        if st.startswith('@prefix') or st.startswith('@base'):
            continue
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
            if any(w in pred.lower() for w in prose_words):
                skipped_prose+=1; continue
            for obj in split_top(objstr, ','):
                obj=obj.strip()
                if not obj: continue
                raw_triples += 1
                if len(triples) < triple_limit:
                    triples.append({'file':rel,'subject':subj,'predicate':pred,'object':obj})
                if pred in keep_props:
                    t=terms.setdefault(subj, {'id':subj,'file':rel,'props':{}})
                    vals=t['props'].setdefault(pred, [])
                    if len(vals) < 8:
                        vals.append(obj)
terms_list=[]
for _,t in sorted(terms.items()):
    if len(terms_list) >= term_limit: break
    terms_list.append({'id':t['id'],'file':t['file'],'props':t['props']})
data={
 'schema':'science.qudt.ontology.v1',
 'source':'QUDT schema/vocab RDF/Turtle structural metadata',
 'license':'CC-BY-4.0',
 'ref':receipt.get('ref','unknown'),
 'archive_sha256':receipt.get('archive_sha256',''),
 'scope':'official src/main/rdf/schema + src/main/rdf/vocab TTL; prose predicates/examples/validation/build excluded',
 'limits':{'triple_limit':triple_limit,'term_limit':term_limit,'raw_triples_seen':raw_triples,'prose_predicates_skipped':skipped_prose},
 'source_files':source_files,
 'prefixes':prefixes[:500],
 'triples':triples,
 'terms':terms_list,
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: files={len(source_files)} triples={len(triples)}/{raw_triples} terms={len(terms_list)}/{len(terms)} skipped_prose={skipped_prose} bytes={out.stat().st_size}")
PY
