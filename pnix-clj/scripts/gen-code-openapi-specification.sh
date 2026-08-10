#!/usr/bin/env bash
# OpenAPI official schema JSON snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${OPENAPI_SPEC_SRC:-$ROOT/ingest/code/openapi-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/openapi-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing OpenAPI schema snapshot: $SRC" >&2
  echo "run scripts/update-code-openapi-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
PROSE_KEYS={'description','title','$comment','markdownDescription','examples','default'}
def walk(node,path,rows,refs,required_rows):
    if isinstance(node,dict):
        if '$ref' in node: refs.append({'path':path,'ref':node.get('$ref')})
        if 'required' in node and isinstance(node['required'],list):
            for x in node['required']: required_rows.append({'path':path,'required':x})
        props=node.get('properties')
        if isinstance(props,dict):
            for name,val in props.items():
                rows.append({'path':path,'property':name,'type':val.get('type') if isinstance(val,dict) else None,'ref':val.get('$ref') if isinstance(val,dict) else None,'format':val.get('format') if isinstance(val,dict) else None})
        pats=node.get('patternProperties')
        if isinstance(pats,dict):
            for pat,val in pats.items(): rows.append({'path':path,'pattern_property':pat,'ref':val.get('$ref') if isinstance(val,dict) else None})
        defs=node.get('$defs') or node.get('definitions')
        if isinstance(defs,dict):
            for name,val in defs.items(): rows.append({'path':path,'definition':name,'type':val.get('type') if isinstance(val,dict) else None})
        for k,v in node.items():
            if k in PROSE_KEYS: continue
            walk(v, path+'/'+k.replace('/','~1'), rows, refs, required_rows)
    elif isinstance(node,list):
        for i,v in enumerate(node): walk(v, path+'/'+str(i), rows, refs, required_rows)
files=[]; schema_terms=[]; refs=[]; required=[]
for f in receipt.get('files',[]):
    data=json.loads((src/f['relative_path']).read_text(encoding='utf-8'))
    file_row={'family':f['family'],'kind':f['kind'],'version':f['version'],'schema_id':data.get('$id') or data.get('id'),'schema_dialect':data.get('$schema'),'relative_path':f['relative_path'],'source_url':f['url'],'sha256':f['sha256'],'size_bytes':f['size_bytes']}
    files.append(file_row)
    local_terms=[]; local_refs=[]; local_required=[]
    walk(data,'#',local_terms,local_refs,local_required)
    for r in local_terms:
        r.update({'family':f['family'],'kind':f['kind'],'version':f['version']})
    for r in local_refs:
        r.update({'family':f['family'],'kind':f['kind'],'version':f['version']})
    for r in local_required:
        r.update({'family':f['family'],'kind':f['kind'],'version':f['version']})
    schema_terms.extend(local_terms[:900])
    refs.extend(local_refs[:600])
    required.extend(local_required[:600])
obj={'schema':'api.openapi.specification.v1','source':{'name':'OpenAPI Initiative official OpenAPI Specification schemas','license':'Apache-2.0','source_urls':['https://github.com/OAI/OpenAPI-Specification','https://spec.openapis.org/oas/'],'receipt':receipt,'generator':'scripts/gen-code-openapi-specification.sh','scope':'official schema structure only; spec prose, real API documents, secrets/server URLs/logs/execution and graph/mirror wiring excluded'},'summary':{'schema_file_count':len(files),'schema_term_count':len(schema_terms),'ref_count':len(refs),'required_field_count':len(required),'specification_prose_ingested':False,'real_api_documents_ingested':False,'api_keys_or_secrets_ingested':False,'server_urls_or_request_logs_ingested':False,'execution_or_invocation_enabled':False,'write_delete_effect_enabled':False,'mirror_graph_wiring':False},'schema_files':files,'schema_terms':schema_terms,'refs':refs,'required_fields':required}
def pnix(v, indent=0):
    import json
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/openapi-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-openapi-specification.sh && scripts/gen-code-openapi-specification.sh\n'
content+='# 범위: OpenAPI official schema structure only. spec prose/real API docs/secrets/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(files)} terms={len(schema_terms)} refs={len(refs)} required={len(required)} bytes={len(content.encode())}')
PY
