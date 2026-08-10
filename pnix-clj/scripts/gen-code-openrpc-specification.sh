#!/usr/bin/env bash
# OpenRPC spec snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${OPENRPC_SPEC_SRC:-$ROOT/ingest/code/openrpc-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/openrpc-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing OpenRPC snapshot: $SRC" >&2
  echo "run scripts/update-code-openrpc-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; definitions=[]; properties=[]; required=[]; refs=[]; enums=[]; table_rows=[]; tokens=[]; headings=[]
seen_refs=set(); seen_tok=set(); TOKEN_RE=re.compile(r'`([^`\n]{1,96})`')
def add_ref(source_path,path,value):
    if not isinstance(value,str) or not value.startswith('#/'): return
    key=(source_path,path,value)
    if key not in seen_refs:
        seen_refs.add(key); refs.append({'source_path':source_path,'path':path,'ref':value})
def walk_schema(node,path,source_path):
    if isinstance(node,dict):
        if '$ref' in node: add_ref(source_path,path,node.get('$ref'))
        if 'enum' in node and isinstance(node['enum'],list):
            for v in node['enum'][:80]:
                if isinstance(v,(str,int,float,bool)) or v is None:
                    enums.append({'source_path':source_path,'path':path,'value':v})
        if path.endswith('/definitions'):
            for name, sub in sorted((node or {}).items()):
                if isinstance(sub,dict):
                    definitions.append({'source_path':source_path,'name':name,'type':sub.get('type'), 'additional_properties':sub.get('additionalProperties') if isinstance(sub.get('additionalProperties'),bool) else None})
        if 'properties' in node and isinstance(node['properties'],dict):
            req=set(node.get('required') or []) if isinstance(node.get('required'),list) else set()
            for name, prop in sorted(node['properties'].items()):
                if isinstance(prop,dict):
                    properties.append({'source_path':source_path,'schema_path':path,'property':name,'type':prop.get('type'), 'format':prop.get('format'), 'required':name in req, 'ref':prop.get('$ref') if isinstance(prop.get('$ref'),str) else None})
        if 'required' in node and isinstance(node['required'],list):
            for r in node['required']:
                if isinstance(r,str): required.append({'source_path':source_path,'schema_path':path,'field':r})
        for k,v in node.items():
            if k in ('description','examples','example','externalDocs','summary'): continue
            walk_schema(v,path+'/'+k,source_path)
    elif isinstance(node,list):
        for i,v in enumerate(node[:500]): walk_schema(v,path+f'[{i}]',source_path)
for f in receipt.get('files',[]):
    if f.get('role') not in ('json_schema','spec_doc'): continue
    path=f['source_path']; rel=f['relative_path']; p=src/rel
    source_files.append({'source_path':path,'role':f['role'],'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    if f['role']=='json_schema':
        data=json.loads(p.read_text(encoding='utf-8'))
        walk_schema(data,'#',path)
    else:
        text=p.read_text(encoding='utf-8',errors='replace'); section=''; in_fence=False
        for line_no,line in enumerate(text.splitlines(),1):
            s=line.strip()
            if s.startswith('```'):
                in_fence=not in_fence; continue
            if in_fence: continue
            if s.startswith('#'):
                title=s.lstrip('#').strip()
                if len(title)<=100:
                    section=title; headings.append({'source_path':path,'line':line_no,'level':len(s)-len(s.lstrip('#')),'heading':title})
                continue
            if s.startswith('|') and s.endswith('|'):
                cells=[c.strip() for c in s.strip('|').split('|')]
                if not cells or all(set(c.replace(':','').replace('-','').strip()) <= {'-'} for c in cells): continue
                joined=' '.join(cells)
                if len(joined)<=340 and all(len(c)<=130 for c in cells):
                    table_rows.append({'source_path':path,'line':line_no,'section':section,'cells':cells})
            for m in TOKEN_RE.finditer(line):
                tok=m.group(1).strip()
                if not tok or len(tok)>96 or '://' in tok or tok.count(' ')>2: continue
                key=(path,tok)
                if key not in seen_tok:
                    seen_tok.add(key); tokens.append({'source_path':path,'section':section,'token':tok})
obj={'schema':'api.openrpc.specification.v1','source':{'name':'OpenRPC official specification schema','license':'Apache-2.0','source_urls':['https://github.com/open-rpc/spec','https://github.com/open-rpc/spec/releases/tag/v1.4.1'],'receipt':receipt,'generator':'scripts/gen-code-openrpc-specification.sh','scope':'official schema structural metadata only; prose fields/examples/real OpenRPC docs/endpoints/credentials/logs/invocation/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'definition_count':len(definitions),'property_count':len(properties),'required_count':len(required),'ref_count':len(refs),'enum_count':len(enums),'table_row_count':len(table_rows),'token_count':len(tokens),'heading_count':len(headings),'schema_description_or_example_ingested':False,'real_openrpc_document_ingested':False,'endpoint_or_credentials_ingested':False,'request_response_logs_ingested':False,'json_rpc_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'definitions':definitions,'properties':properties,'required':required,'refs':refs,'enums':enums,'table_rows':table_rows[:160],'tokens':tokens[:500],'section_headings':headings[:180]}
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
content='# stdlib/lib/corpus/openrpc-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-openrpc-specification.sh && scripts/gen-code-openrpc-specification.sh\n'
content+='# 범위: OpenRPC official schema 구조 메타데이터만. prose/examples/endpoints/invocation/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(source_files)} definitions={len(definitions)} properties={len(properties)} required={len(required)} refs={len(refs)} enums={len(enums)} tables={min(len(table_rows),160)}/{len(table_rows)} tokens={min(len(tokens),500)}/{len(tokens)} bytes={len(content.encode())}')
PY
