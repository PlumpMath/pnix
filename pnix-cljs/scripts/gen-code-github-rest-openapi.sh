#!/usr/bin/env bash
# GitHub REST OpenAPI snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${GITHUB_REST_OPENAPI_SRC:-$ROOT/ingest/code/github-rest-openapi}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/github-rest-openapi.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing GitHub REST OpenAPI snapshot: $SRC" >&2
  echo "run scripts/update-code-github-rest-openapi.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_file=next(f for f in receipt['files'] if f.get('role')=='openapi_json')
data=json.loads((src/source_file['relative_path']).read_text(encoding='utf-8'))
operations=[]; operation_params=[]; operation_responses=[]; request_bodies=[]
methods={'get','put','post','delete','patch','head','options','trace'}
for path, item in sorted((data.get('paths') or {}).items()):
    common_params=item.get('parameters') or [] if isinstance(item,dict) else []
    for method, op in sorted((item or {}).items()):
        if method not in methods or not isinstance(op,dict): continue
        op_id=op.get('operationId')
        tags=[t for t in (op.get('tags') or []) if isinstance(t,str)][:8]
        operations.append({'path':path,'method':method.upper(),'operation_id':op_id,'tags':tags,'deprecated':bool(op.get('deprecated',False))})
        params=list(common_params)+list(op.get('parameters') or [])
        for p in params[:40]:
            if not isinstance(p,dict): continue
            if '$ref' in p:
                operation_params.append({'operation_id':op_id,'path':path,'method':method.upper(),'ref':p.get('$ref')})
                continue
            sch=p.get('schema') if isinstance(p.get('schema'),dict) else {}
            operation_params.append({'operation_id':op_id,'path':path,'method':method.upper(),'name':p.get('name'),'in':p.get('in'),'required':bool(p.get('required',False)),'schema_type':sch.get('type'),'schema_ref':sch.get('$ref')})
        rb=op.get('requestBody')
        if isinstance(rb,dict):
            if '$ref' in rb:
                request_bodies.append({'operation_id':op_id,'path':path,'method':method.upper(),'ref':rb.get('$ref')})
            else:
                content=rb.get('content') if isinstance(rb.get('content'),dict) else {}
                request_bodies.append({'operation_id':op_id,'path':path,'method':method.upper(),'required':bool(rb.get('required',False)),'content_types':sorted(content.keys())[:12]})
        for code, resp in sorted((op.get('responses') or {}).items()):
            if isinstance(resp,dict):
                content=resp.get('content') if isinstance(resp.get('content'),dict) else {}
                operation_responses.append({'operation_id':op_id,'path':path,'method':method.upper(),'status':str(code),'ref':resp.get('$ref'),'content_types':sorted(content.keys())[:12]})
components=data.get('components') if isinstance(data.get('components'),dict) else {}
schemas=components.get('schemas') if isinstance(components.get('schemas'),dict) else {}
schema_rows=[]; schema_properties=[]; schema_required=[]; schema_refs=[]; schema_enums=[]
def ref_of(x): return x.get('$ref') if isinstance(x,dict) and isinstance(x.get('$ref'),str) else None
for name, sch in sorted(schemas.items()):
    if not isinstance(sch,dict): continue
    schema_rows.append({'name':name,'type':sch.get('type'),'ref':ref_of(sch),'additional_properties':sch.get('additionalProperties') if isinstance(sch.get('additionalProperties'),bool) else None})
    req=set(sch.get('required') or []) if isinstance(sch.get('required'),list) else set()
    for r in sorted(req): schema_required.append({'schema':name,'field':r})
    props=sch.get('properties') if isinstance(sch.get('properties'),dict) else {}
    for prop, ps in sorted(props.items()):
        if not isinstance(ps,dict): continue
        ref=ref_of(ps)
        items=ps.get('items') if isinstance(ps.get('items'),dict) else {}
        schema_properties.append({'schema':name,'property':prop,'type':ps.get('type'),'format':ps.get('format'),'required':prop in req,'ref':ref,'items_ref':ref_of(items),'items_type':items.get('type')})
        if ref: schema_refs.append({'schema':name,'property':prop,'ref':ref})
        if ref_of(items): schema_refs.append({'schema':name,'property':prop+'[]','ref':ref_of(items)})
        if isinstance(ps.get('enum'),list):
            for v in ps['enum'][:30]: schema_enums.append({'schema':name,'property':prop,'value':v})
parameters=components.get('parameters') if isinstance(components.get('parameters'),dict) else {}
component_parameters=[]
for name,p in sorted(parameters.items()):
    if isinstance(p,dict):
        sch=p.get('schema') if isinstance(p.get('schema'),dict) else {}
        component_parameters.append({'name':name,'in':p.get('in'),'required':bool(p.get('required',False)),'schema_type':sch.get('type'),'schema_ref':sch.get('$ref')})
security_schemes=[]
for name,ss in sorted((components.get('securitySchemes') or {}).items()):
    if isinstance(ss,dict): security_schemes.append({'name':name,'type':ss.get('type'),'scheme':ss.get('scheme'),'in':ss.get('in')})
obj={'schema':'api.github_rest.openapi.v1','source':{'name':'GitHub REST API official OpenAPI description','license':'MIT','source_urls':['https://github.com/github/rest-api-description','https://github.com/github/rest-api-description/tree/v2.1.0'],'receipt':receipt,'generator':'scripts/gen-code-github-rest-openapi.sh','scope':'official OpenAPI structural metadata only; descriptions/examples/API calls/tokens/private data/diff bodies/request logs/execution/graph wiring excluded'},'summary':{'openapi_version':data.get('openapi'),'info_version':(data.get('info') or {}).get('version'),'path_count':len(data.get('paths') or {}),'operation_count':len(operations),'operation_param_count':len(operation_params),'operation_response_count':len(operation_responses),'request_body_count':len(request_bodies),'component_schema_count':len(schema_rows),'schema_property_count_total':len(schema_properties),'schema_property_count_stored':min(len(schema_properties),150),'component_parameter_count':len(component_parameters),'security_scheme_count':len(security_schemes),'descriptions_or_examples_ingested':False,'api_tokens_ingested':False,'private_repo_data_ingested':False,'diff_patch_bodies_ingested':False,'request_response_logs_ingested':False,'api_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':[{'source_path':source_file['source_path'],'sha256':source_file['sha256'],'size_bytes':source_file['size_bytes']}],'operations':operations[:1200],'operation_parameters':operation_params[:150],'operation_responses':operation_responses[:150],'request_bodies':request_bodies[:80],'component_schemas':schema_rows[:1200],'schema_properties':schema_properties[:150],'schema_required':schema_required[:150],'schema_refs':schema_refs[:150],'schema_enums':schema_enums[:50],'component_parameters':component_parameters,'security_schemes':security_schemes}
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
content='# stdlib/lib/corpus/github-rest-openapi.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-github-rest-openapi.sh && scripts/gen-code-github-rest-openapi.sh\n'
content+='# 범위: GitHub REST OpenAPI 구조 메타데이터만. descriptions/examples/tokens/private data/diff bodies/invocation/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: operations={len(operations)} params={min(len(operation_params),150)}/{len(operation_params)} responses={min(len(operation_responses),150)}/{len(operation_responses)} schemas={min(len(schema_rows),1200)}/{len(schema_rows)} properties={min(len(schema_properties),150)}/{len(schema_properties)} bytes={len(content.encode())}')
PY
