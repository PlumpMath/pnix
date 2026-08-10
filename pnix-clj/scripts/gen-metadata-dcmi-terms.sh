#!/usr/bin/env bash
# DCMI Metadata Terms RDF vocabulary -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${DCMI_SRC:-$ROOT/ingest/metadata/dcmi-terms}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/dcmi-terms.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing DCMI terms snapshot: $SRC" >&2
  echo "run scripts/update-metadata-dcmi-terms.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import collections, json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
RDF='{http://www.w3.org/1999/02/22-rdf-syntax-ns#}'; RDFS='{http://www.w3.org/2000/01/rdf-schema#}'; DCT='{http://purl.org/dc/terms/}'
ABOUT=RDF+'about'; RESOURCE=RDF+'resource'
root=ET.parse(src/'raw'/'dublin_core_terms.rdf').getroot()
terms=collections.defaultdict(lambda:{'types':set(),'domains':set(),'ranges':set(),'subproperty_of':set(),'is_defined_by':set(),'issued':None,'modified':None,'label':None,'comment_present':False,'comment_char_count':0})
for desc in root:
    uri=desc.attrib.get(ABOUT)
    if not uri or not (uri.startswith('http://purl.org/dc/terms/') or uri.startswith('http://purl.org/dc/elements/1.1/') or uri.startswith('http://purl.org/dc/dcmitype/')): continue
    t=terms[uri]
    for node in desc:
        tag=node.tag
        res=node.attrib.get(RESOURCE)
        txt=(node.text or '').strip()
        if tag==RDFS+'label' and txt and not t['label']: t['label']=txt
        elif tag==RDFS+'comment' and txt:
            t['comment_present']=True; t['comment_char_count']+=len(txt)
        elif tag==RDF+'type' and res: t['types'].add(res)
        elif tag==RDFS+'domain' and res: t['domains'].add(res)
        elif tag==RDFS+'range' and res: t['ranges'].add(res)
        elif tag==RDFS+'subPropertyOf' and res: t['subproperty_of'].add(res)
        elif tag==RDFS+'isDefinedBy' and res: t['is_defined_by'].add(res)
        elif tag==DCT+'issued' and txt: t['issued']=txt
        elif tag==DCT+'modified' and txt: t['modified']=txt

def ns_local(uri):
    if uri.startswith('http://purl.org/dc/terms/'):
        return ('dcterms',uri.rsplit('/',1)[-1])
    if uri.startswith('http://purl.org/dc/elements/1.1/'):
        return ('dc11',uri.rsplit('/',1)[-1])
    if uri.startswith('http://purl.org/dc/dcmitype/'):
        return ('dcmitype',uri.rsplit('/',1)[-1])
    if '#' in uri:
        return (uri.rsplit('#',1)[0]+'#',uri.rsplit('#',1)[-1])
    return ('',uri.rsplit('/',1)[-1])
rows=[]; type_counts=collections.Counter(); ns_counts=collections.Counter()
for uri,data in sorted(terms.items()):
    ns,local=ns_local(uri); ns_counts[ns]+=1
    types=sorted(data['types'])
    for ty in types: type_counts[ns_local(ty)[1]]+=1
    rows.append({'uri':uri,'namespace':ns,'local_name':local,'label':data['label'],'types':[ns_local(x)[1] for x in types],'domains':sorted(data['domains']),'ranges':sorted(data['ranges']),'subproperty_of':sorted(data['subproperty_of']),'is_defined_by':sorted(data['is_defined_by']),'issued':data['issued'],'modified':data['modified'],'comment_present':data['comment_present'],'comment_char_count':data['comment_char_count']})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')} for f in receipt.get('files') or []]
obj={'schema':'metadata.dcmi.terms.v1','source':{'name':'DCMI Metadata Terms RDF vocabulary','license':'CC-BY-4.0 / DCMI attribution license family','source_urls':['https://www.dublincore.org/specifications/dublin-core/dcmi-terms/','https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.rdf','https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.ttl','https://www.dublincore.org/about/copyright/'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-metadata-dcmi-terms.sh','scope':'term URI/type/domain/range/label/date structure only; rdfs:comment prose excluded'},'summary':{'term_count':len(rows),'namespace_counts':[{'namespace':k,'count':v} for k,v in sorted(ns_counts.items())],'type_counts':[{'type':k,'count':v} for k,v in sorted(type_counts.items())],'comment_prose_ingested':False,'comment_presence_counts_ingested':True,'specification_prose_ingested':False,'examples_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'terms':rows}
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
content='# stdlib/lib/corpus/dcmi-terms.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-metadata-dcmi-terms.sh && scripts/gen-metadata-dcmi-terms.sh\n'
content+='# 범위: DCMI term URI/type/domain/range/label/date structure only. comment prose/spec prose/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: terms={len(rows)} bytes={len(content.encode())}')
PY
