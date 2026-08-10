#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
YEAR="${MESH_YEAR:-2026}"
IN="$ROOT/ingest/clinical/mesh-core"
OUT="$ROOT/stdlib/lib/corpus/mesh-core.generated.px"
DESC_LIMIT="${MESH_DESCRIPTOR_LIMIT:-900}"
QUAL_LIMIT="${MESH_QUALIFIER_LIMIT:-200}"
python3 - "$IN/desc${YEAR}.gz" "$IN/qual${YEAR}.xml" "$OUT" "$YEAR" "$DESC_LIMIT" "$QUAL_LIMIT" <<'PY'
import gzip, html, sys, xml.etree.ElementTree as ET
from pathlib import Path

desc_path, qual_path, out_path, year, desc_limit, qual_limit = sys.argv[1:]
desc_limit=int(desc_limit); qual_limit=int(qual_limit)

def text(el, path):
    x=el.find(path)
    return (x.text or '').strip() if x is not None and x.text else None

def texts(el, path, limit=None):
    vals=[]
    for x in el.findall(path):
        if x.text and x.text.strip(): vals.append(x.text.strip())
        if limit and len(vals)>=limit: break
    return vals

def esc(s):
    return '"' + s.replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ') + '"'

def pnix(v):
    if isinstance(v, bool): return 'true' if v else 'false'
    if isinstance(v, int): return str(v)
    if isinstance(v, str): return esc(v)
    if isinstance(v, list): return '[ ' + ' '.join(pnix(x) for x in v) + ' ]'
    if isinstance(v, dict): return '{ ' + ' '.join(f'{k} = {pnix(val)};' for k,val in v.items()) + ' }'
    if v is None: return 'null'
    raise TypeError(type(v))

descriptors=[]; total_desc=0
with gzip.open(desc_path, 'rb') as fh:
    for ev, el in ET.iterparse(fh, events=('end',)):
        if el.tag.endswith('DescriptorRecord'):
            total_desc += 1
            if len(descriptors) < desc_limit:
                ui=text(el,'DescriptorUI') or ''
                name=text(el,'DescriptorName/String') or ''
                trees=texts(el,'TreeNumberList/TreeNumber', 16)
                quals=[]
                for q in el.findall('AllowableQualifiersList/AllowableQualifier')[:32]:
                    q_ui=text(q,'QualifierReferredTo/QualifierUI') or ''
                    q_name=text(q,'QualifierReferredTo/QualifierName/String') or ''
                    if q_ui or q_name: quals.append({'ui':q_ui,'name':q_name})
                concepts=[]
                for c in el.findall('ConceptList/Concept')[:8]:
                    cui=text(c,'ConceptUI') or ''
                    cname=text(c,'ConceptName/String') or ''
                    preferred=(c.attrib.get('PreferredConceptYN') == 'Y')
                    terms=[]
                    for t in c.findall('TermList/Term')[:10]:
                        tui=text(t,'TermUI') or ''
                        tname=text(t,'String') or ''
                        if tui or tname: terms.append({'ui':tui,'name':tname})
                    concepts.append({'ui':cui,'name':cname,'preferred':preferred,'terms':terms})
                descriptors.append({'ui':ui,'name':name,'tree_numbers':trees,'allowable_qualifiers':quals,'concepts':concepts})
            el.clear()
qualifiers=[]; total_qual=0
for ev, el in ET.iterparse(qual_path, events=('end',)):
    if el.tag.endswith('QualifierRecord'):
        total_qual += 1
        if len(qualifiers) < qual_limit:
            ui=text(el,'QualifierUI') or ''
            name=text(el,'QualifierName/String') or ''
            abbr=text(el,'Abbreviation') or ''
            trees=texts(el,'TreeNumberList/TreeNumber', 16)
            qualifiers.append({'ui':ui,'name':name,'abbreviation':abbr,'tree_numbers':trees})
        el.clear()
seed={
 'schema':'clinical.mesh_core.v1',
 'source':{'name':'NLM Medical Subject Headings (MeSH) core','year':year,'license':'NLM Terms and Conditions','attribution':'Courtesy of the U.S. National Library of Medicine','source_urls':['https://www.nlm.nih.gov/databases/download/mesh.html']},
 'summary':{'descriptor_count_total':total_desc,'descriptor_count_stored':len(descriptors),'qualifier_count_total':total_qual,'qualifier_count_stored':len(qualifiers),'scope_note_prose_ingested':False,'translations_ingested':False,'scr_ingested':False,'pubmed_records_ingested':False,'mirror_graph_wiring':False},
 'descriptors':descriptors,
 'qualifiers':qualifiers
}
Path(out_path).write_text(pnix(seed)+'\n')
print(f'generated {out_path}: descriptors={len(descriptors)}/{total_desc}, qualifiers={len(qualifiers)}/{total_qual}')
PY
