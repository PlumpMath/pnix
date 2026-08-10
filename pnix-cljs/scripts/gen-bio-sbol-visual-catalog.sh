#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/bio/sbol-visual-catalog"
OUT="$ROOT/stdlib/lib/corpus/sbol-visual-catalog.generated.px"
python3 - "$IN" "$OUT" <<'PY'
import json, sys, xml.etree.ElementTree as ET
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2])

def esc(s): return '"'+s.replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))

tree=json.loads((root/'tree.json').read_text())
glyph_paths=[]
for x in tree.get('tree',[]):
    p=x.get('path','')
    if x.get('type')=='blob' and p.startswith('Glyphs/'):
        ext=p.rsplit('.',1)[-1].lower() if '.' in p else ''
        parts=p.split('/')
        if ext in {'svg','pdf','png','md'}:
            glyph_paths.append({'path':p,'category':parts[1] if len(parts)>2 else '', 'name':parts[2] if len(parts)>3 else '', 'kind':ext, 'payload_stored':False})
# RDF structure: store subject uri/local type and predicate object refs/literals bounded, no prose values for comments.
ns={'rdf':'http://www.w3.org/1999/02/22-rdf-syntax-ns#','rdfs':'http://www.w3.org/2000/01/rdf-schema#','owl':'http://www.w3.org/2002/07/owl#'}
root_xml=ET.parse(root/'sbol-vo.rdf').getroot()
subjects=[]
for child in list(root_xml)[:500]:
    subj=child.attrib.get('{%s}about'%ns['rdf']) or child.attrib.get('{%s}ID'%ns['rdf']) or ''
    pred_refs=[]
    labels=[]
    for e in list(child):
        tag=e.tag.split('}',1)[-1]
        if tag in {'comment','description','definition'}: continue
        res=e.attrib.get('{%s}resource'%ns['rdf'])
        if tag=='label' and e.text and len(labels)<4: labels.append(e.text.strip())
        elif res and len(pred_refs)<40: pred_refs.append({'predicate':tag,'object':res})
    if subj or labels or pred_refs:
        subjects.append({'subject':subj,'element':child.tag.split('}',1)[-1],'labels':labels,'refs':pred_refs})
seed={'schema':'bio.sbol_visual_catalog.v1','source':{'name':'SynBioDex SBOL Visual catalog','license':'CC0 glyphs / CC-BY-4.0 non-glyph repository work','repo':'https://github.com/SynBioDex/SBOL-visual'},'summary':{'glyph_path_count':len(glyph_paths),'rdf_subject_count':len(subjects),'artwork_payloads_ingested':False,'specification_prose_ingested':False,'examples_ingested':False,'scripts_ingested':False,'mirror_graph_wiring':False},'glyph_paths':glyph_paths,'rdf_subjects':subjects}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: glyph_paths={len(glyph_paths)} rdf_subjects={len(subjects)}')
PY
