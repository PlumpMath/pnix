#!/usr/bin/env bash
# AsyncAPI official schema snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ASYNCAPI_SPEC_SRC:-$ROOT/ingest/code/asyncapi-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/asyncapi-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing AsyncAPI schema snapshot: $SRC" >&2
  echo "run scripts/update-code-asyncapi-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
PROSE_KEYS={'description','title','$comment','markdownDescription','examples','default'}
def walk(node,path,terms,refs,required):
    if isinstance(node,dict):
        if '$ref' in node: refs.append({'path':path,'ref':node.get('$ref')})
        if 'required' in node and isinstance(node['required'],list):
            for x in node['required']: required.append({'path':path,'required':x})
        props=node.get('properties')
        if isinstance(props,dict):
            for name,val in props.items(): terms.append({'path':path,'property':name,'type':val.get('type') if isinstance(val,dict) else None,'ref':val.get('$ref') if isinstance(val,dict) else None,'format':val.get('format') if isinstance(val,dict) else None})
        defs=node.get('definitions') or node.get('$defs')
        if isinstance(defs,dict):
            for name,val in defs.items(): terms.append({'path':path,'definition':name,'type':val.get('type') if isinstance(val,dict) else None})
        pats=node.get('patternProperties')
        if isinstance(pats,dict):
            for pat,val in pats.items(): terms.append({'path':path,'pattern_property':pat,'ref':val.get('$ref') if isinstance(val,dict) else None})
        for k,v in node.items():
            if k in PROSE_KEYS: continue
            walk(v,path+'/'+k.replace('/','~1'),terms,refs,required)
    elif isinstance(node,list):
        for i,v in enumerate(node): walk(v,path+'/'+str(i),terms,refs,required)
files=[]; terms=[]; refs=[]; required=[]
for f in receipt.get('files',[]):
    if f.get('role')!='schema_json': continue
    data=json.loads((src/f['relative_path']).read_text(encoding='utf-8'))
    version=f['source_path'].split('/')[-1].replace('.json','')
    files.append({'source_path':f['source_path'],'schema_id':data.get('$id') or data.get('id'),'schema_dialect':data.get('$schema'),'version':version,'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    lt=[]; lr=[]; lq=[]; walk(data,'#',lt,lr,lq)
    for r in lt[:420]: r.update({'version':version,'source_path':f['source_path']}); terms.append(r)
    for r in lr[:260]: r.update({'version':version,'source_path':f['source_path']}); refs.append(r)
    for r in lq[:160]: r.update({'version':version,'source_path':f['source_path']}); required.append(r)
bindings=json.loads((src/'binding-catalog.json').read_text(encoding='utf-8'))
obj={'schema':'api.asyncapi.specification.v1','source':{'name':'AsyncAPI official specification JSON schemas','license':'Apache-2.0','source_urls':['https://github.com/asyncapi/spec','https://github.com/asyncapi/spec-json-schemas'],'receipt':receipt,'generator':'scripts/gen-code-asyncapi-specification.sh','scope':'official schema structure and binding catalog only; spec prose, real AsyncAPI docs, broker credentials/logs/execution and graph/mirror wiring excluded'},'summary':{'schema_file_count':len(files),'schema_term_count':len(terms),'ref_count':len(refs),'required_field_count':len(required),'binding_count':len(bindings),'specification_prose_ingested':False,'real_asyncapi_documents_ingested':False,'broker_urls_credentials_logs_ingested':False,'message_payload_logs_ingested':False,'execution_or_invocation_enabled':False,'publish_subscribe_enabled':False,'mirror_graph_wiring':False},'schema_files':files,'schema_terms':terms,'refs':refs,'required_fields':required,'bindings':bindings}
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
content='# stdlib/lib/corpus/asyncapi-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-asyncapi-specification.sh && scripts/gen-code-asyncapi-specification.sh\n'
content+='# 범위: AsyncAPI official schema structure + binding catalog only. prose/real docs/credentials/logs/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(files)} terms={len(terms)} refs={len(refs)} required={len(required)} bindings={len(bindings)} bytes={len(content.encode())}')
PY
