#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/education/credential-engine-ctdl/raw"
OUT="$ROOT/stdlib/lib/corpus/credential-engine-ctdl.generated.px"
RECEIPT="$ROOT/ingest/education/credential-engine-ctdl/source-receipt.json"
if [[ ! -d "$SRC" ]]; then echo "missing $SRC; run update first" >&2; exit 1; fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt=json.loads(Path(sys.argv[3]).read_text())
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
def val(x):
    if isinstance(x, dict):
        if '@id' in x: return x['@id']
        if 'en-US' in x: return x['en-US']
        if 'en' in x: return x['en']
        return json.dumps(x, ensure_ascii=False, sort_keys=True)
    if isinstance(x, list): return [val(y) for y in x]
    return x
ctx=json.loads((src/'context-json.json').read_text())['@context']
enc=json.loads((src/'encoding-json.json').read_text())
graph=enc.get('@graph', [])
source_files=[{'file':p.name,'bytes':p.stat().st_size} for p in sorted(src.glob('*.json'))]
context=[]
for k,v in sorted(ctx.items()):
    if isinstance(v,str): context.append({'prefix':k,'iri':v})
    elif isinstance(v,dict): context.append({'prefix':k,'iri':v.get('@id',''),'type':v.get('@type',''),'container':v.get('@container','')})
terms=[]; type_counts={}; skipped_prose=0
keep=['@id','@type','rdfs:label','rdfs:subClassOf','rdfs:subPropertyOf','rdfs:domain','rdfs:range','schema:domainIncludes','schema:rangeIncludes','owl:equivalentClass','owl:equivalentProperty','vs:term_status','meta:domainFor','meta:rangeFor','meta:targetScheme']
for item in graph:
    if not isinstance(item,dict) or '@id' not in item: continue
    typ=val(item.get('@type',''))
    if isinstance(typ,list):
        for t in typ: type_counts[t]=type_counts.get(t,0)+1
    elif typ:
        type_counts[typ]=type_counts.get(typ,0)+1
    props={}
    for k in keep:
        if k in item: props[k.replace(':','_').replace('@','at_')]=val(item[k])
    if 'rdfs:comment' in item or 'dcterms:description' in item or 'skos:definition' in item:
        skipped_prose+=1
    terms.append({'id':item['@id'],'type':typ,'props':props})
data={'schema':'education.credential_engine.ctdl_schema.v1','source':'Credential Engine CTDL context/encoding schema metadata','license':'CC-BY-4.0','source_receipt':receipt,'source_files':source_files,'context_terms':context,'terms':terms,'type_counts':[{'type':k,'count':v} for k,v in sorted(type_counts.items())],'summary':{'term_count':len(terms),'context_count':len(context),'prose_terms_skipped_fields':skipped_prose},'exclusions':['rdfs:comment/descriptions','handbook prose','guidance','examples','Credential Registry records','credential/person/org data','API keys','execution','mirror/graph wiring']}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: context={len(context)} terms={len(terms)} prose_fields_excluded={skipped_prose} bytes={out.stat().st_size}")
PY
