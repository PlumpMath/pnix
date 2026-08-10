#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/code/coveragepy-shape/raw"
OUT="$ROOT/stdlib/lib/corpus/coveragepy-shape.generated.px"
RECEIPT="$ROOT/ingest/code/coveragepy-shape/source-receipt.json"
if [[ ! -d "$SRC" ]]; then echo "missing $SRC; run update first" >&2; exit 1; fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import ast,json,sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt=json.loads(Path(sys.argv[3]).read_text())
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
files=[]; classes=[]; functions=[]; constants=[]; typed_dicts=[]
for path in sorted(src.glob('*.py')):
    text=path.read_text(errors='replace'); rel=path.name.replace('__','/')
    files.append({'file':rel,'bytes':path.stat().st_size,'lines':len(text.splitlines())})
    tree=ast.parse(text)
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef):
            bases=[getattr(b,'id',getattr(b,'attr','')) for b in node.bases]
            classes.append({'file':rel,'name':node.name,'line':node.lineno,'bases':[b for b in bases if b]})
            if 'TypedDict' in bases: typed_dicts.append({'file':rel,'name':node.name,'line':node.lineno})
        elif isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef)):
            functions.append({'file':rel,'name':node.name,'line':node.lineno})
        elif isinstance(node, ast.Assign):
            for t in node.targets:
                if isinstance(t, ast.Name) and t.id.isupper(): constants.append({'file':rel,'name':t.id,'line':node.lineno})
data={'schema':'code.coveragepy.shape.v1','source':'coverage.py report/data module structural identifiers','license':'Apache-2.0','ref':receipt.get('ref','unknown'),'archive_sha256':receipt.get('archive_sha256',''),'source_files':files,'classes':classes,'functions':functions,'constants':constants,'typed_dicts':typed_dicts,'report_formats':[{'name':x} for x in ['json','xml','html','lcov','text']],'exclusions':['source bodies','docstrings/prose','coverage reports','measured source paths','line data','test logs','execution','mirror/graph wiring']}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: files={len(files)} classes={len(classes)} functions={len(functions)} constants={len(constants)} typed_dicts={len(typed_dicts)} bytes={out.stat().st_size}")
PY
