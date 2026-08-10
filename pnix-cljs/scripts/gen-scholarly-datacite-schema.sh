#!/usr/bin/env bash
# DataCite XSD snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${DATACITE_SCHEMA_SRC:-$ROOT/ingest/scholarly/datacite-metadata-schema}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/datacite-metadata-schema.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing DataCite schema snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-datacite-schema.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; schema_files=[]; includes=[]; elements=[]; complex_types=[]; simple_types=[]; attributes=[]; enumerations=[]
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
            elements.append({'source_path':path,'name':node.attrib.get('name'),'type':node.attrib.get('type'),'ref':node.attrib.get('ref'),'min_occurs':node.attrib.get('minOccurs'),'max_occurs':node.attrib.get('maxOccurs')})
        elif tag=='complexType':
            complex_types.append({'source_path':path,'name':node.attrib.get('name'),'mixed':node.attrib.get('mixed')=='true'})
        elif tag=='simpleType':
            simple_types.append({'source_path':path,'name':node.attrib.get('name')})
        elif tag=='attribute':
            attributes.append({'source_path':path,'name':node.attrib.get('name'),'type':node.attrib.get('type'),'use':node.attrib.get('use')})
    current_simple=None
    for node in root.iter():
        tag=lname(node.tag)
        if tag=='simpleType': current_simple=node.attrib.get('name')
        elif tag=='enumeration':
            val=node.attrib.get('value')
            if val is not None: enumerations.append({'source_path':path,'simple_type':current_simple,'value':val})
        elif tag=='element' and 'ref' in node.attrib:
            elements.append({'source_path':path,'name':node.attrib.get('name'),'ref':node.attrib.get('ref'),'type':node.attrib.get('type'),'min_occurs':node.attrib.get('minOccurs'),'max_occurs':node.attrib.get('maxOccurs')})
        elif tag=='attribute' and ('ref' in node.attrib or 'name' in node.attrib):
            row={'source_path':path,'name':node.attrib.get('name'),'ref':node.attrib.get('ref'),'type':node.attrib.get('type'),'use':node.attrib.get('use')}
            if row not in attributes: attributes.append(row)
obj={'schema':'scholarly.datacite.metadata_schema.v1','source':{'name':'DataCite Metadata Schema XSD metadata','license':'CC0-1.0 for DataCite metadata','source_urls':['https://github.com/datacite/schema','https://support.datacite.org/docs/harvesting-datacite-doi-metadata'],'receipt':receipt,'generator':'scripts/gen-scholarly-datacite-schema.sh','scope':'official XSD schema structure only; DOI records/title/abstract/person values/landing URLs/resource payloads/API harvest results/graph wiring excluded'},'summary':{'xsd_file_count':len(source_files),'schema_file_count':len(schema_files),'include_count':len(includes),'element_count':len(elements),'complex_type_count':len(complex_types),'simple_type_count':len(simple_types),'attribute_count':len(attributes),'enumeration_count':len(enumerations),'doi_records_ingested':False,'titles_abstracts_descriptions_ingested':False,'personal_metadata_values_ingested':False,'landing_page_or_resource_payloads_ingested':False,'api_harvest_results_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'schema_files':schema_files,'includes':includes,'elements':elements[:700],'complex_types':complex_types[:700],'simple_types':simple_types[:400],'attributes':attributes[:700],'enumerations':enumerations[:1200]}
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
content='# stdlib/lib/corpus/datacite-metadata-schema.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-datacite-schema.sh && scripts/gen-scholarly-datacite-schema.sh\n'
content+='# 범위: DataCite XSD 구조 메타데이터만. DOI records/prose/person values/API harvest/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: xsd={len(source_files)} elements={min(len(elements),700)}/{len(elements)} complex={min(len(complex_types),700)}/{len(complex_types)} simple={min(len(simple_types),400)}/{len(simple_types)} enums={min(len(enumerations),1200)}/{len(enumerations)} bytes={len(content.encode())}')
PY
