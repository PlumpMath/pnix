#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${WORKFLOW_OPEN_STANDARDS_SRC:-$ROOT/ingest/workflow/open-standards-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/workflow-open-standards-catalog.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then echo "missing snapshot: $SRC" >&2; exit 2; fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text())
wdl_spec=(src/'raw/wdl/SPEC.md').read_text(encoding='utf-8',errors='ignore')
headings=[]; code_blocks=[]; tokens=[]; in_code=False; lang=None; buf=[]
for line in wdl_spec.splitlines():
    m=re.match(r'^(#{1,6})\s+(.+)$',line)
    if m: headings.append({'level':len(m.group(1)),'title':m.group(2).strip()[:120]})
    m=re.match(r'^```\s*([A-Za-z0-9_.+-]*)',line)
    if m and not in_code: in_code=True; lang=m.group(1) or 'plain'; buf=[]; continue
    if line.startswith('```') and in_code:
        text='\n'.join(buf)
        code_blocks.append({'language':lang,'line_count':len(buf),'first_line':buf[0][:120] if buf else ''})
        for t in re.findall(r'\b[A-Za-z_][A-Za-z0-9_]{2,}\b', text)[:40]:
            if t not in {'the','and','for','with','this','that','string','object'}: tokens.append({'source':'wdl_code_block','token':t})
        in_code=False; continue
    if in_code: buf.append(line)
ro_terms=[]
for p in sorted((src/'raw/ro-crate').glob('*.json*')):
    f=str(p.relative_to(src))
    try:
        data=json.loads(p.read_text(encoding='utf-8'))
    except Exception:
        continue
    def walk(x,path):
        if isinstance(x,dict):
            for k,v in x.items():
                if k in {'description','name'} and isinstance(v,str) and len(v)>80: continue
                ro_terms.append({'file':f,'path':'.'.join(path+[str(k)])[:180],'kind':'object' if isinstance(v,dict) else 'array' if isinstance(v,list) else 'scalar'})
                walk(v,path+[str(k)])
        elif isinstance(x,list):
            for i,v in enumerate(x[:20]): walk(v,path+[str(i)])
    walk(data,[])
obj={'schema':'workflow.open_standards_catalog.v1','source':{'name':'WDL and RO-Crate workflow/FAIR schema catalog','license':'BSD-3-Clause / Apache-2.0','source_urls':['https://github.com/openwdl/wdl','https://github.com/ResearchObject/ro-crate'],'receipt':receipt,'generator':'scripts/gen-workflow-open-standards-catalog.sh','scope':'structural metadata only; prose/examples/workflow payloads/command lines/data files/execution/graph wiring excluded'},'summary':{'wdl_heading_count':len(headings),'wdl_code_block_count':len(code_blocks),'wdl_token_count':len(tokens),'ro_crate_term_count':len(ro_terms),'prose_bodies_ingested':False,'examples_ingested':False,'workflow_or_crate_payloads_ingested':False,'command_payloads_ingested':False,'runtime_execution_enabled':False,'mirror_graph_wiring':False},'wdl':{'headings':headings[:160],'code_blocks':code_blocks[:160],'tokens':tokens[:500]},'ro_crate':{'terms':ro_terms[:700]}}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list): return '[ ]' if not v else '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict): return '{ }' if not v else '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/workflow-open-standards-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-workflow-open-standards-catalog.sh && scripts/gen-workflow-open-standards-catalog.sh\n'
content+='# 범위: WDL/RO-Crate structural metadata only. prose/examples/payloads/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: wdl_headings={len(headings)} code_blocks={len(code_blocks)} ro_terms={len(ro_terms)} bytes={len(content.encode())}')
PY
