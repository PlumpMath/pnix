#!/usr/bin/env bash
# CWL schema YAML snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CWL_SCHEMA_SRC:-$ROOT/ingest/workflow/cwl-schema-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/cwl-schema-catalog.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing CWL schema snapshot: $SRC" >&2
  echo "run scripts/update-workflow-cwl-schema-catalog.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
rows=[]; fields=[]; enums=[]; imports=[]
for f in receipt.get('files',[]):
    rel=f['relative_path']; text=(src/rel).read_text(encoding='utf-8',errors='ignore')
    current=None
    for line in text.splitlines():
        m=re.match(r'^-\s+name:\s*([A-Za-z0-9_.:-]+)\s*$', line)
        if m:
            current={'file':rel,'name':m.group(1),'kind':None}; rows.append(current); continue
        if current:
            m=re.match(r'^\s+type:\s*([A-Za-z0-9_.:-]+)', line)
            if m and not current.get('kind'): current['kind']=m.group(1)
            m=re.match(r'^\s+extends:\s*([A-Za-z0-9_.:-]+)', line)
            if m: current['extends']=m.group(1)
        m=re.match(r'^\s+([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$', line)
        if current and m and m.group(1) in {'fields','symbols','inputs','outputs','requirements','hints'}:
            current.setdefault('sections',[]).append(m.group(1))
        m=re.match(r'^\s*-\s+([A-Za-z_][A-Za-z0-9_]*):\s*$', line)
        if current and m:
            fields.append({'file':rel,'owner':current['name'],'field':m.group(1)})
        m=re.match(r'^\s*-\s+([A-Za-z][A-Za-z0-9_.:-]+)\s*$', line)
        if current and m and len(enums)<500:
            val=m.group(1)
            if val not in {'name','type','fields'}: enums.append({'file':rel,'owner':current['name'],'symbol':val})
        m=re.match(r'^\$import:\s*(\S+)', line)
        if m: imports.append({'file':rel,'import':m.group(1)})
classes=[r for r in rows if r.get('name')]
obj={'schema':'workflow.cwl.schema_catalog.v1','source':{'name':'Common Workflow Language v1.2 schema catalog','license':'Apache-2.0','source_urls':['https://github.com/common-workflow-language/cwl-v1.2'],'receipt':receipt,'generator':'scripts/gen-workflow-cwl-schema-catalog.sh','scope':'official schema structure only; spec prose/tests/examples/workflow payloads/execution/graph wiring excluded'},'summary':{'file_count':len(receipt.get('files') or []),'class_count':len(classes),'field_token_count':len(fields),'symbol_token_count':len(enums),'import_count':len(imports),'specification_prose_ingested':False,'examples_ingested':False,'workflow_payloads_ingested':False,'command_payloads_ingested':False,'runtime_execution_enabled':False,'mirror_graph_wiring':False},'schema_files':receipt.get('files') or [],'classes':classes[:400],'field_tokens':fields[:700],'symbol_tokens':enums[:700],'imports':imports}
def pnix(v, indent=0):
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
content='# stdlib/lib/corpus/cwl-schema-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-workflow-cwl-schema-catalog.sh && scripts/gen-workflow-cwl-schema-catalog.sh\n'
content+='# 범위: CWL schema structure only. prose/tests/examples/workflow payloads/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: classes={len(classes)} fields={len(fields)} symbols={len(enums)} bytes={len(content.encode())}')
PY
