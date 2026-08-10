#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/code/pytest-result-vocab/raw"
OUT="$ROOT/stdlib/lib/corpus/pytest-result-vocab.generated.px"
RECEIPT="$ROOT/ingest/code/pytest-result-vocab/source-receipt.json"
if [[ ! -d "$SRC" ]]; then echo "missing $SRC; run update first" >&2; exit 1; fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import ast,json,re,sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt=json.loads(Path(sys.argv[3]).read_text())
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
files=[]; classes=[]; functions=[]; assignments=[]; outcomes=set(); marks=set(); options=[]
for path in sorted(src.glob('*.py')):
    text=path.read_text(errors='replace'); rel=path.name.replace('__','/')
    files.append({'file':rel,'bytes':path.stat().st_size,'lines':len(text.splitlines())})
    tree=ast.parse(text)
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef): classes.append({'file':rel,'name':node.name,'line':node.lineno})
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)): functions.append({'file':rel,'name':node.name,'line':node.lineno})
        elif isinstance(node, ast.Assign):
            for t in node.targets:
                if isinstance(t, ast.Name) and t.id in ('skip','fail','xfail'):
                    assignments.append({'file':rel,'name':t.id,'line':node.lineno})
        elif isinstance(node, ast.Constant) and isinstance(node.value,str):
            if node.value in ('passed','failed','skipped'): outcomes.add(node.value)
    for m in re.finditer(r'addinivalue_line\(\s*"markers"\s*,\s*"([A-Za-z_][\w]*)', text):
        marks.add(m.group(1))
    for m in re.finditer(r'parser\.addoption\(\s*"(--[A-Za-z0-9_-]+)"', text):
        options.append({'file':rel,'name':m.group(1)})
# include conventional terminal summary outcome words used by pytest ecosystem
for x in ['passed','failed','skipped','xfailed','xpassed','error','rerun']:
    outcomes.add(x)
data={'schema':'code.pytest.result_vocab.v1','source':'pytest outcome/report/mark structural identifiers','license':'MIT','ref':receipt.get('ref','unknown'),'archive_sha256':receipt.get('archive_sha256',''),'source_files':files,'classes':classes,'functions':functions,'public_helpers':assignments,'outcomes':[{'name':x} for x in sorted(outcomes)],'registered_marks':[{'name':x} for x in sorted(marks)],'options':options,'exclusions':['source bodies','docstrings/prose','tests','test logs','user results','configs','execution','mirror/graph wiring']}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: files={len(files)} classes={len(classes)} functions={len(functions)} outcomes={len(outcomes)} marks={len(marks)} options={len(options)} bytes={out.stat().st_size}")
PY
