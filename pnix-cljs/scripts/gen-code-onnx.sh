#!/usr/bin/env bash
# ONNX snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ONNX_SRC:-$ROOT/ingest/code/onnx}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/onnx.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing ONNX snapshot: $SRC" >&2
  echo "run scripts/update-code-onnx.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; proto_files=[]; imports=[]; messages=[]; enums=[]; fields=[]; doc_headings=[]; doc_tokens=[]; operators=[]
seen_tok=set(); TOKEN_RE=re.compile(r'`([^`\n]{1,96})`')
STRIP_COMMENTS=re.compile(r'//.*?$|/\*.*?\*/', re.S|re.M)
for f in receipt.get('files',[]):
    if f.get('role') not in ('proto_schema','spec_doc'): continue
    path=f['source_path']; rel=f['relative_path']; p=src/rel
    source_files.append({'source_path':path,'role':f['role'],'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    if f['role']=='proto_schema':
        text=p.read_text(encoding='utf-8',errors='replace')
        no=STRIP_COMMENTS.sub('',text)
        package=(re.search(r'\bpackage\s+([A-Za-z0-9_.]+)\s*;',no) or [None,None])[1]
        syntax=(re.search(r'\bsyntax\s*=\s*"([^"]+)"\s*;',no) or [None,None])[1]
        proto_files.append({'source_path':path,'package':package,'syntax':syntax})
        for m in re.finditer(r'\bimport\s+"([^"]+)"\s*;',no): imports.append({'source_path':path,'import':m.group(1)})
        for m in re.finditer(r'\bmessage\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no): messages.append({'source_path':path,'package':package,'message':m.group(1)})
        for m in re.finditer(r'\benum\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no): enums.append({'source_path':path,'package':package,'enum':m.group(1)})
        field_re=re.compile(r'(?m)^\s*(optional|required|repeated)?\s*([A-Za-z_][A-Za-z0-9_.<>]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([0-9]+)')
        for m in field_re.finditer(no):
            typ=m.group(2); name=m.group(3)
            if typ in ('option','reserved','extensions','message','enum','service','rpc','returns'): continue
            fields.append({'source_path':path,'package':package,'label':m.group(1) or 'singular','type':typ,'name':name,'number':int(m.group(4))})
    else:
        text=p.read_text(encoding='utf-8',errors='replace'); section=''; in_fence=False
        for line_no,line in enumerate(text.splitlines(),1):
            s=line.strip()
            if s.startswith('```'):
                in_fence=not in_fence; continue
            if in_fence: continue
            if s.startswith('#'):
                level=len(s)-len(s.lstrip('#'))
                title=s.lstrip('#').strip()
                if len(title)<=120:
                    doc_headings.append({'source_path':path,'line':line_no,'level':level,'heading':title})
                    section=title
                    # Operators.md headings usually contain op names. Keep bounded op catalog rows.
                    if path.endswith('Operators.md') and (level in (2,3,4)):
                        raw=title.split(' - ')[0].strip()
                        m=re.match(r'^([A-Za-z_][A-Za-z0-9_]*)(?:\s*-\s*(\d+))?', raw)
                        if m:
                            operators.append({'source_path':path,'line':line_no,'name':m.group(1),'since_version':int(m.group(2)) if m.group(2) else None,'heading':title[:120]})
                continue
            for mt in TOKEN_RE.finditer(line):
                tok=mt.group(1).strip()
                if not tok or len(tok)>96 or '://' in tok or tok.count(' ')>2: continue
                key=(path,tok)
                if key not in seen_tok:
                    seen_tok.add(key); doc_tokens.append({'source_path':path,'section':section,'token':tok})
obj={'schema':'ml.onnx.specification.v1','source':{'name':'ONNX official IR and operator specification metadata','license':'Apache-2.0','source_urls':['https://github.com/onnx/onnx','https://github.com/onnx/onnx/tree/v1.22.0'],'receipt':receipt,'generator':'scripts/gen-code-onnx.sh','scope':'official proto/operator/IR structural metadata only; prose/examples/model files/weights/datasets/runtime execution/generated code/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'proto_file_count':len(proto_files),'import_count':len(imports),'message_count':len(messages),'enum_count':len(enums),'field_count':len(fields),'operator_heading_count':len(operators),'doc_heading_count':len(doc_headings),'doc_token_count':len(doc_tokens),'prose_bodies_ingested':False,'examples_ingested':False,'model_files_or_weights_ingested':False,'datasets_or_training_artifacts_ingested':False,'runtime_execution_enabled':False,'generated_code_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'proto_files':proto_files,'imports':imports,'messages':messages,'enums':enums,'fields':fields[:900],'operators':operators[:500],'doc_headings':doc_headings[:500],'doc_tokens':doc_tokens[:800]}
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
content='# stdlib/lib/corpus/onnx.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-onnx.sh && scripts/gen-code-onnx.sh\n'
content+='# 범위: ONNX official proto/operator/IR 구조 메타데이터만. prose/examples/models/weights/runtime/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: proto_files={len(proto_files)} messages={len(messages)} enums={len(enums)} fields={min(len(fields),900)}/{len(fields)} operators={min(len(operators),500)}/{len(operators)} doc_tokens={min(len(doc_tokens),800)}/{len(doc_tokens)} bytes={len(content.encode())}')
PY
