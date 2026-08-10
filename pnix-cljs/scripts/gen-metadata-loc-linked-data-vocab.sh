#!/usr/bin/env bash
# LoC Linked Data selected vocabulary/root schemes -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${LOC_LD_SRC:-$ROOT/ingest/metadata/loc-linked-data-vocab}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/loc-linked-data-vocab.generated.px}"
MEMBER_LIMIT="${LOC_LD_MEMBER_LIMIT:-1000}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing LoC linked-data vocab snapshot: $SRC" >&2
  echo "run scripts/update-metadata-loc-linked-data-vocab.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$MEMBER_LIMIT" <<'PY'
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); limit=int(sys.argv[3])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
RDF='{http://www.w3.org/1999/02/22-rdf-syntax-ns#}'; RDFS='{http://www.w3.org/2000/01/rdf-schema#}'; MADS='{http://www.loc.gov/mads/rdf/v1#}'; SKOS='{http://www.w3.org/2004/02/skos/core#}'
ABOUT=RDF+'about'; RESOURCE=RDF+'resource'
def txt(node): return (node.text or '').strip() if node is not None else None
schemes=[]; members=[]
for f in receipt.get('files') or []:
    p=src/f.get('relative_path')
    root=ET.parse(p).getroot()
    for scheme in root.findall(MADS+'MADSScheme'):
        uri=scheme.attrib.get(ABOUT)
        label=txt(scheme.find(RDFS+'label')) or txt(scheme.find(MADS+'authoritativeLabel')) or txt(scheme.find(SKOS+'prefLabel'))
        comment=txt(scheme.find(RDFS+'comment'))
        defn=txt(scheme.find(MADS+'definitionNote'))
        mem_nodes=scheme.findall(MADS+'hasMADSSchemeMember')
        schemes.append({'source_path':f.get('source_path'),'scheme_uri':uri,'label':label,'member_count':len(mem_nodes),'comment_present':bool(comment),'comment_char_count':len(comment or ''),'definition_note_present':bool(defn),'definition_note_char_count':len(defn or '')})
        for mn in mem_nodes:
            res=mn.attrib.get(RESOURCE)
            if res:
                if len(members)<limit: members.append({'scheme_uri':uri,'member_uri':res})
                continue
            for child in mn:
                res=child.attrib.get(ABOUT) or child.attrib.get(RESOURCE)
                label=txt(child.find(RDFS+'label')) or txt(child.find(MADS+'authoritativeLabel')) or txt(child.find(SKOS+'prefLabel'))
                code=txt(child.find(MADS+'code'))
                if len(members)<limit: members.append({'scheme_uri':uri,'member_uri':res,'code':code,'label':label})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')} for f in receipt.get('files') or []]
obj={'schema':'metadata.loc.linked_data_vocab.v1','source':{'name':'Library of Congress Linked Data selected vocabulary/root schemes','license':'LoC public domain / public authority data','source_urls':['https://id.loc.gov/','https://id.loc.gov/vocabulary/relators.rdf','https://id.loc.gov/vocabulary/identifiers.rdf','https://id.loc.gov/authorities/subjects.rdf'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-metadata-loc-linked-data-vocab.sh','scope':'scheme/member URI/code/label structure only; authority payloads and prose notes excluded'},'summary':{'scheme_count':len(schemes),'member_sample_count':len(members),'member_limit':limit,'authority_record_payloads_ingested':False,'personal_name_records_ingested':False,'bibliographic_records_ingested':False,'comment_definition_prose_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'schemes':schemes,'members':members}
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
content='# stdlib/lib/corpus/loc-linked-data-vocab.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-metadata-loc-linked-data-vocab.sh && scripts/gen-metadata-loc-linked-data-vocab.sh\n'
content+='# 범위: LoC scheme/member URI/code/label structure only. authority payloads/prose/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: schemes={len(schemes)} members={len(members)} bytes={len(content.encode())}')
PY
