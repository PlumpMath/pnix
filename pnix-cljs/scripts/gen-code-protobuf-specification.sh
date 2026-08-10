#!/usr/bin/env bash
# Protocol Buffers core .proto snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${PROTOBUF_SPEC_SRC:-$ROOT/ingest/code/protobuf-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/protobuf-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing protobuf snapshot: $SRC" >&2
  echo "run scripts/update-code-protobuf-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
STRIP_COMMENTS=re.compile(r'//.*?$|/\*.*?\*/', re.S|re.M)
file_rows=[]; messages=[]; enums=[]; services=[]; rpcs=[]; fields=[]; imports=[]
for f in receipt.get('files',[]):
    if not f.get('source_path','').endswith('.proto'): continue
    text=(src/f['relative_path']).read_text(encoding='utf-8',errors='replace')
    no=STRIP_COMMENTS.sub('',text)
    package=(re.search(r'\bpackage\s+([A-Za-z0-9_.]+)\s*;',no) or [None,None])[1]
    syntax=(re.search(r'\bsyntax\s*=\s*"([^"]+)"\s*;',no) or [None,None])[1]
    file_rows.append({'source_path':f['source_path'],'package':package,'syntax':syntax,'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    for m in re.finditer(r'\bimport\s+(?:public\s+|weak\s+)?"([^"]+)"\s*;',no):
        imports.append({'source_path':f['source_path'],'import':m.group(1)})
    for m in re.finditer(r'\bmessage\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no):
        messages.append({'source_path':f['source_path'],'package':package,'message':m.group(1)})
    for m in re.finditer(r'\benum\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no):
        enums.append({'source_path':f['source_path'],'package':package,'enum':m.group(1)})
    for m in re.finditer(r'\bservice\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no):
        services.append({'source_path':f['source_path'],'package':package,'service':m.group(1)})
    for m in re.finditer(r'\brpc\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((stream\s+)?([A-Za-z0-9_.]+)\)\s+returns\s+\((stream\s+)?([A-Za-z0-9_.]+)\)',no):
        rpcs.append({'source_path':f['source_path'],'package':package,'rpc':m.group(1),'client_streaming':bool(m.group(2)),'input_type':m.group(3),'server_streaming':bool(m.group(4)),'output_type':m.group(5)})
    field_re=re.compile(r'(?m)^\s*(optional|required|repeated)?\s*([A-Za-z_][A-Za-z0-9_.<>]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([0-9]+)')
    for m in field_re.finditer(no):
        typ=m.group(2); name=m.group(3)
        if typ in ('option','reserved','extensions','message','enum','service','rpc','returns'): continue
        fields.append({'source_path':f['source_path'],'package':package,'label':m.group(1) or 'singular','type':typ,'name':name,'number':int(m.group(4))})
obj={'schema':'code.protobuf.specification.v1','source':{'name':'Protocol Buffers official core .proto schema files','license':'BSD-style permissive license','source_urls':['https://github.com/protocolbuffers/protobuf','https://github.com/protocolbuffers/protobuf/releases'],'receipt':receipt,'generator':'scripts/gen-code-protobuf-specification.sh','scope':'official core .proto structure only; tests/generated code/runtime/source/descriptor registry/customer protos/payloads/execution and graph/mirror wiring excluded'},'summary':{'proto_file_count':len(file_rows),'import_count':len(imports),'message_count':len(messages),'enum_count':len(enums),'service_count':len(services),'rpc_count':len(rpcs),'field_count':len(fields),'test_protos_ingested':False,'generated_code_ingested':False,'production_descriptor_sets_ingested':False,'schema_registry_or_customer_proto_ingested':False,'message_payloads_or_logs_ingested':False,'code_generation_or_invocation_enabled':False,'mirror_graph_wiring':False},'proto_files':file_rows,'imports':imports,'messages':messages,'enums':enums,'services':services,'rpcs':rpcs,'fields':fields[:1200]}
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
content='# stdlib/lib/corpus/protobuf-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-protobuf-specification.sh && scripts/gen-code-protobuf-specification.sh\n'
content+='# 범위: Protobuf official core .proto structure only. tests/generated/runtime/descriptor registry/customer protos/payloads/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(file_rows)} messages={len(messages)} enums={len(enums)} services={len(services)} rpcs={len(rpcs)} fields={min(len(fields),1200)}/{len(fields)} bytes={len(content.encode())}')
PY
