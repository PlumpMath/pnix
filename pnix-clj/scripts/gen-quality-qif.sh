#!/usr/bin/env bash
# QIF XSD snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${QIF_SRC:-$ROOT/ingest/quality/qif}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/qif.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing QIF snapshot: $SRC" >&2
  echo "run scripts/update-quality-qif.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
XS='{http://www.w3.org/2001/XMLSchema}'
source_files=[]; schema_files=[]; includes=[]; elements=[]; complex_types=[]; simple_types=[]; attributes=[]; groups=[]; attribute_groups=[]
def lname(x): return x.split('}',1)[-1] if isinstance(x,str) else x
for f in receipt.get('files',[]):
    if f.get('role')!='xsd_schema': continue
    path=f['source_path']; p=src/f['relative_path']
    source_files.append({'source_path':path,'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    root=ET.parse(p).getroot()
    schema_files.append({'source_path':path,'target_namespace':root.attrib.get('targetNamespace'),'version':root.attrib.get('version'),'element_form_default':root.attrib.get('elementFormDefault'),'attribute_form_default':root.attrib.get('attributeFormDefault')})
    for node in list(root):
        tag=lname(node.tag)
        if tag in ('include','import'):
            includes.append({'source_path':path,'kind':tag,'schema_location':node.attrib.get('schemaLocation'),'namespace':node.attrib.get('namespace')})
        elif tag=='element':
            elements.append({'source_path':path,'name':node.attrib.get('name'),'type':node.attrib.get('type'),'substitution_group':node.attrib.get('substitutionGroup'),'abstract':node.attrib.get('abstract')=='true'})
        elif tag=='complexType':
            complex_types.append({'source_path':path,'name':node.attrib.get('name'),'mixed':node.attrib.get('mixed')=='true','abstract':node.attrib.get('abstract')=='true'})
        elif tag=='simpleType':
            simple_types.append({'source_path':path,'name':node.attrib.get('name')})
        elif tag=='attribute':
            attributes.append({'source_path':path,'name':node.attrib.get('name'),'type':node.attrib.get('type'),'use':node.attrib.get('use')})
        elif tag=='group':
            groups.append({'source_path':path,'name':node.attrib.get('name')})
        elif tag=='attributeGroup':
            attribute_groups.append({'source_path':path,'name':node.attrib.get('name')})
    # nested attribute/element refs: structural references only, no docs.
    for node in root.iter():
        tag=lname(node.tag)
        if tag=='attribute' and ('ref' in node.attrib or 'name' in node.attrib):
            row={'source_path':path,'name':node.attrib.get('name'),'ref':node.attrib.get('ref'),'type':node.attrib.get('type'),'use':node.attrib.get('use')}
            if row not in attributes: attributes.append(row)
        elif tag=='element' and ('ref' in node.attrib):
            elements.append({'source_path':path,'name':node.attrib.get('name'),'ref':node.attrib.get('ref'),'type':node.attrib.get('type'),'min_occurs':node.attrib.get('minOccurs'),'max_occurs':node.attrib.get('maxOccurs')})
obj={'schema':'quality.qif.xsd_schema.v1','source':{'name':'Quality Information Framework qif-community XML schema metadata','license':'BSL-1.0; CodeSynthesis binding paths excluded','source_urls':['https://github.com/QualityInformationFramework/qif-community','https://qifstandards.org/about-qif/'],'receipt':receipt,'generator':'scripts/gen-quality-qif.sh','scope':'Boost-licensed CPP-Kramer XSD structure only; CodeSynthesis bindings/generated source/sample instances/measurement values/process guidance/execution/graph wiring excluded'},'summary':{'xsd_file_count':len(source_files),'schema_file_count':len(schema_files),'include_count':len(includes),'top_level_element_count':len(elements),'complex_type_count':len(complex_types),'simple_type_count':len(simple_types),'attribute_count':len(attributes),'group_count':len(groups),'attribute_group_count':len(attribute_groups),'codesynthesis_binding_paths_ingested':False,'sample_instance_files_ingested':False,'measurement_values_ingested':False,'manufacturing_process_guidance_ingested':False,'execution_or_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'schema_files':schema_files,'includes':includes,'elements':elements[:900],'complex_types':complex_types[:900],'simple_types':simple_types[:600],'attributes':attributes[:900],'groups':groups,'attribute_groups':attribute_groups}
def pnix(v,indent=0):
    import json
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v),ensure_ascii=False)
content='# stdlib/lib/corpus/qif.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-quality-qif.sh && scripts/gen-quality-qif.sh\n'
content+='# 범위: QIF CPP-Kramer XSD 구조 메타데이터만. CodeSynthesis/sample values/process guidance/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: xsd={len(source_files)} elements={min(len(elements),900)}/{len(elements)} complex={min(len(complex_types),900)}/{len(complex_types)} simple={min(len(simple_types),600)}/{len(simple_types)} attrs={min(len(attributes),900)}/{len(attributes)} bytes={len(content.encode())}')
PY
