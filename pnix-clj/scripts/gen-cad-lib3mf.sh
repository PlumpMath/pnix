#!/usr/bin/env bash
# lib3mf schema snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${LIB3MF_SRC:-$ROOT/ingest/cad/lib3mf}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/lib3mf.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing lib3mf snapshot: $SRC" >&2
  echo "run scripts/update-cad-lib3mf.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
XS='{http://www.w3.org/2001/XMLSchema}'
source_files=[]; xsd_files=[]; includes=[]; elements=[]; complex_types=[]; simple_types=[]; attributes=[]; parse_errors=[]; interface_packages=[]; interface_classes=[]; interface_methods=[]; interface_enums=[]
def lname(x): return x.split('}',1)[-1] if isinstance(x,str) else x
for f in receipt.get('files',[]):
    if f.get('role') not in ('xsd_schema','interface_xml'): continue
    path=f['source_path']; p=src/f['relative_path']
    source_files.append({'source_path':path,'role':f['role'],'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    if f['role']=='xsd_schema':
        
        try:
            root=ET.parse(p).getroot()
        except ET.ParseError as e:
            raw=p.read_text(encoding='utf-8-sig',errors='replace')
            # lib3mf v2.5.0 beamlattice_2017_02.xsd has malformed XML. Keep source bytes unchanged.
            raw=raw.replace('namespace"targetNamespace', 'namespace" targetNamespace')
            try:
                root=ET.fromstring(raw)
            except ET.ParseError as e2:
                parse_errors.append({'source_path':path,'error':str(e2)})
                continue
        xsd_files.append({'source_path':path,'target_namespace':root.attrib.get('targetNamespace'),'version':root.attrib.get('version'),'element_form_default':root.attrib.get('elementFormDefault'),'attribute_form_default':root.attrib.get('attributeFormDefault')})
        for node in list(root):
            tag=lname(node.tag)
            if tag in ('include','import'):
                includes.append({'source_path':path,'kind':tag,'schema_location':node.attrib.get('schemaLocation'),'namespace':node.attrib.get('namespace')})
            elif tag=='element':
                elements.append({'source_path':path,'name':node.attrib.get('name'),'type':node.attrib.get('type'),'ref':node.attrib.get('ref')})
            elif tag=='complexType':
                complex_types.append({'source_path':path,'name':node.attrib.get('name'),'mixed':node.attrib.get('mixed')=='true'})
            elif tag=='simpleType':
                simple_types.append({'source_path':path,'name':node.attrib.get('name')})
            elif tag=='attribute':
                attributes.append({'source_path':path,'name':node.attrib.get('name'),'type':node.attrib.get('type'),'use':node.attrib.get('use')})
        for node in root.iter():
            tag=lname(node.tag)
            if tag=='element' and 'ref' in node.attrib:
                elements.append({'source_path':path,'name':node.attrib.get('name'),'ref':node.attrib.get('ref'),'type':node.attrib.get('type'),'min_occurs':node.attrib.get('minOccurs'),'max_occurs':node.attrib.get('maxOccurs')})
            elif tag=='attribute' and ('ref' in node.attrib or 'name' in node.attrib):
                row={'source_path':path,'name':node.attrib.get('name'),'ref':node.attrib.get('ref'),'type':node.attrib.get('type'),'use':node.attrib.get('use')}
                if row not in attributes: attributes.append(row)
obj={'schema':'cad.lib3mf.schema.v1','source':{'name':'3MF Consortium lib3mf schema metadata','license':'BSD-2-Clause','source_urls':['https://github.com/3MFConsortium/lib3mf','https://github.com/3MFConsortium/lib3mf/tree/v2.5.0'],'receipt':receipt,'generator':'scripts/gen-cad-lib3mf.sh','scope':'interface XML and XSD structure only; model payloads/geometry values/printer parameters/generated bindings/prose docs/execution/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'xsd_file_count':len(xsd_files),'include_count':len(includes),'element_count':len(elements),'complex_type_count':len(complex_types),'simple_type_count':len(simple_types),'attribute_count':len(attributes),'parse_error_count':len(parse_errors),'interface_package_count':len(interface_packages),'interface_class_count':len(interface_classes),'interface_method_count':len(interface_methods),'interface_enum_count':len(interface_enums),'model_payloads_ingested':False,'mesh_geometry_values_ingested':False,'printer_process_parameters_ingested':False,'generated_bindings_ingested':False,'execution_or_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'xsd_files':xsd_files,'includes':includes,'elements':elements[:700],'complex_types':complex_types[:700],'simple_types':simple_types[:300],'attributes':attributes[:500],'parse_errors':parse_errors,'interface_packages':interface_packages,'interface_classes':interface_classes[:400],'interface_methods':interface_methods[:700],'interface_enums':interface_enums[:300]}
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
content='# stdlib/lib/corpus/lib3mf.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-cad-lib3mf.sh && scripts/gen-cad-lib3mf.sh\n'
content+='# 범위: lib3mf interface XML + XSD 구조 메타데이터만. model payloads/geometry/process/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: xsd={len(xsd_files)} elements={min(len(elements),700)}/{len(elements)} complex={min(len(complex_types),700)}/{len(complex_types)} interface_classes={min(len(interface_classes),400)}/{len(interface_classes)} methods={min(len(interface_methods),700)}/{len(interface_methods)} bytes={len(content.encode())}')
PY
