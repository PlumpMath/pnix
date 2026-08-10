#!/usr/bin/env bash
# GraphQL spec snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${GRAPHQL_SPEC_SRC:-$ROOT/ingest/code/graphql-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/graphql-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing GraphQL snapshot: $SRC" >&2
  echo "run scripts/update-code-graphql-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; grammar=[]; tokens=[]; tables=[]; headings=[]; metadata_keys=[]
TOKEN_RE=re.compile(r'`([^`\n]{1,96})`')
PROD_RE=re.compile(r'^([A-Z][A-Za-z0-9_]*)(\s*::|\s*:)\s*(.*)$')
seen_tok=set(); in_fence=False; current_prod=None; section=''
for f in receipt.get('files',[]):
    if f.get('role') not in ('language_type_validation_spec_doc','metadata'): continue
    path=f['source_path']; rel=f['relative_path']; p=src/rel
    source_files.append({'source_path':path,'role':f['role'],'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    if path.endswith('.json'):
        data=json.loads(p.read_text(encoding='utf-8'))
        def walk(x,prefix=''):
            if isinstance(x,dict):
                for k,v in sorted(x.items()):
                    key=(prefix+'.'+k).strip('.')
                    if len(key)<=120: metadata_keys.append({'source_path':path,'key':key})
                    walk(v,key)
            elif isinstance(x,list):
                for i,v in enumerate(x[:20]): walk(v,prefix+f'[{i}]')
        walk(data)
        continue
    text=p.read_text(encoding='utf-8',errors='replace')
    current_prod=None
    for line_no,line in enumerate(text.splitlines(),1):
        raw=line.rstrip('\n'); s=raw.strip()
        if s.startswith('```'):
            in_fence=not in_fence
            current_prod=None
            continue
        if in_fence:
            continue
        if s.startswith('#'):
            title=s.lstrip('#').strip()
            if len(title)<=100:
                section=title; headings.append({'source_path':path,'line':line_no,'level':len(s)-len(s.lstrip('#')),'heading':title})
            current_prod=None
            continue
        m=PROD_RE.match(s)
        if m:
            lhs=m.group(1); op=m.group(2).strip(); rhs=m.group(3).strip()
            current_prod={'source_path':path,'line':line_no,'section':section,'lhs':lhs,'operator':op,'rhs':rhs,'alternatives':[ ]}
            grammar.append(current_prod)
            continue
        if current_prod is not None and s.startswith('- '):
            alt=s[2:].strip()
            if len(alt)<=160:
                current_prod['alternatives'].append(alt)
            continue
        if s.startswith('|') and s.endswith('|'):
            cells=[c.strip() for c in s.strip('|').split('|')]
            if not cells or all(set(c.replace(':','').replace('-','').strip()) <= {'-'} for c in cells): continue
            joined=' '.join(cells)
            if len(joined)<=320 and all(len(c)<=120 for c in cells):
                tables.append({'source_path':path,'line':line_no,'section':section,'cells':cells})
        for mt in TOKEN_RE.finditer(raw):
            tok=mt.group(1).strip()
            if not tok or len(tok)>96 or '://' in tok or tok.count(' ')>2: continue
            key=(path,tok)
            if key not in seen_tok:
                seen_tok.add(key); tokens.append({'source_path':path,'section':section,'token':tok})
operation_types=[{'operation_type':'query'},{'operation_type':'mutation'},{'operation_type':'subscription'}]
type_system_kinds=[{'kind':x} for x in ['schema','scalar','object','interface','union','enum','input_object','directive']]
obj={'schema':'api.graphql.specification.v1','source':{'name':'GraphQL official specification documents','license':'OWFa-1.0 for specifications','source_urls':['https://github.com/graphql/graphql-spec','https://github.com/graphql/graphql-spec/releases/tag/September2025'],'receipt':receipt,'generator':'scripts/gen-code-graphql-specification.sh','scope':'official language/type-system/validation structural metadata only; prose/examples/introspection schema export/prod schema dumps/request logs/endpoints/credentials/execution/invocation/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'grammar_production_count':len(grammar),'token_count':len(tokens),'table_row_count':len(tables),'heading_count':len(headings),'metadata_key_count':len(metadata_keys),'spec_prose_body_ingested':False,'examples_ingested':False,'introspection_schema_export_ingested':False,'production_schema_dump_ingested':False,'request_response_logs_ingested':False,'execution_or_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'grammar_productions':grammar,'operation_types':operation_types,'type_system_kinds':type_system_kinds,'tokens':tokens[:900],'table_rows':tables[:180],'section_headings':headings[:240],'metadata_keys':metadata_keys}
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
content='# stdlib/lib/corpus/graphql-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-graphql-specification.sh && scripts/gen-code-graphql-specification.sh\n'
content+='# 범위: GraphQL official 구조 메타데이터만. prose/examples/introspection dump/request logs/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(source_files)} grammar={len(grammar)} tokens={min(len(tokens),900)}/{len(tokens)} tables={min(len(tables),180)}/{len(tables)} headings={min(len(headings),240)}/{len(headings)} metadata_keys={len(metadata_keys)} bytes={len(content.encode())}')
PY
