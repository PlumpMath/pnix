#!/usr/bin/env bash
# gRPC core protocol docs snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${GRPC_SPEC_SRC:-$ROOT/ingest/code/grpc-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/grpc-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing gRPC snapshot: $SRC" >&2
  echo "run scripts/update-code-grpc-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
file_rows=[]; table_rows=[]; tokens=[]
seen_tokens=set()
TOKEN_RE=re.compile(r'`([^`\n]{1,96})`')
for f in receipt.get('files',[]):
    if f.get('role')!='core_protocol_doc':
        continue
    path=f['source_path']; rel=f['relative_path']; text=(src/rel).read_text(encoding='utf-8',errors='replace')
    file_rows.append({'source_path':path,'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    section=''
    for line_no,line in enumerate(text.splitlines(),1):
        s=line.strip()
        if s.startswith('#'):
            title=s.lstrip('#').strip()
            if len(title) <= 80:
                section=title
            continue
        if s.startswith('|') and s.endswith('|'):
            cells=[c.strip() for c in s.strip('|').split('|')]
            if not cells or all(set(c.replace(':','').replace('-','').strip()) <= {'-'} for c in cells):
                continue
            joined=' '.join(cells)
            if len(joined) <= 360 and all(len(c) <= 140 for c in cells):
                table_rows.append({'source_path':path,'line':line_no,'section':section,'cells':cells})
        for m in TOKEN_RE.finditer(line):
            tok=m.group(1).strip()
            if not tok or len(tok)>96 or '://' in tok or '\t' in tok:
                continue
            if tok.count(' ') > 2:
                continue
            key=(path,tok)
            if key not in seen_tokens:
                seen_tokens.add(key)
                tokens.append({'source_path':path,'section':section,'token':tok})
stream_types=[
    {'stream_type':'unary','client_streaming':False,'server_streaming':False},
    {'stream_type':'client_streaming','client_streaming':True,'server_streaming':False},
    {'stream_type':'server_streaming','client_streaming':False,'server_streaming':True},
    {'stream_type':'bidirectional_streaming','client_streaming':True,'server_streaming':True},
]
obj={'schema':'api.grpc.specification.v1','source':{'name':'gRPC official core protocol specification documents','license':'Apache-2.0','source_urls':['https://github.com/grpc/grpc','https://github.com/grpc/grpc/releases','https://github.com/grpc/grpc/tree/master/doc'],'receipt':receipt,'generator':'scripts/gen-code-grpc-specification.sh','scope':'official core protocol structural metadata only; spec prose body/server reflection/prod schema export/customer descriptors/payload logs/live endpoints/credentials/invocation/graph wiring excluded'},'summary':{'source_file_count':len(file_rows),'table_row_count':len(table_rows),'token_count':len(tokens),'stream_type_count':len(stream_types),'spec_prose_body_ingested':False,'server_reflection_ingested':False,'production_schema_export_ingested':False,'customer_descriptor_or_payload_ingested':False,'live_endpoint_or_credentials_ingested':False,'rpc_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':file_rows,'stream_types':stream_types,'table_rows':table_rows[:240],'tokens':tokens[:800]}
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
content='# stdlib/lib/corpus/grpc-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-grpc-specification.sh && scripts/gen-code-grpc-specification.sh\n'
content+='# 범위: gRPC official core protocol 구조 메타데이터만. prose/server-reflection/prod schema/live endpoint/invocation/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(file_rows)} table_rows={min(len(table_rows),240)}/{len(table_rows)} tokens={min(len(tokens),800)}/{len(tokens)} bytes={len(content.encode())}')
PY
