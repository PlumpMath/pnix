#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/code/cucumber-gherkin/raw"
OUT="$ROOT/stdlib/lib/corpus/cucumber-gherkin.generated.px"
RECEIPT="$ROOT/ingest/code/cucumber-gherkin/source-receipt.json"
if [[ ! -d "$SRC" ]]; then echo "missing $SRC; run update first" >&2; exit 1; fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json,re,sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt=json.loads(Path(sys.argv[3]).read_text())
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
berp=(src/'gherkin.berp').read_text(errors='replace')
langs=json.loads((src/'gherkin-languages.json').read_text())
files=[{'file':'gherkin.berp','bytes':(src/'gherkin.berp').stat().st_size,'lines':len(berp.splitlines())},{'file':'gherkin-languages.json','bytes':(src/'gherkin-languages.json').stat().st_size,'dialect_count':len(langs)}]
rules=[]; tokens=[]
for i,line in enumerate(berp.splitlines(),1):
    s=line.strip()
    if not s or s.startswith('#'): continue
    m=re.match(r'^([A-Za-z_][\w]*)\s*:\s*(.*)$', s)
    if m:
        name,body=m.groups()
        rules.append({'name':name,'line':i,'body_tokens':re.findall(r'[A-Za-z_][\w]*|\'[^\']*\'|"[^"]*"|[|*+?()]', body)})
        for tok in re.findall(r'[A-Za-z_][\w]*', body):
            if tok not in tokens: tokens.append(tok)

dialects=[]; keyword_rows=[]
for code,info in sorted(langs.items()):
    dialects.append({'code':code,'name':info.get('name',''),'native':info.get('native','')})
    for kind,vals in sorted(info.items()):
        if isinstance(vals,list):
            for v in vals:
                keyword_rows.append({'dialect':code,'keyword_type':kind,'keyword':v})
data={'schema':'code.cucumber_gherkin.grammar.v1','source':'cucumber/gherkin root grammar and dialect keywords','license':'MIT','ref':receipt.get('ref','unknown'),'archive_sha256':receipt.get('archive_sha256',''),'source_files':files,'grammar_rules':rules,'grammar_tokens':[{'name':t} for t in sorted(tokens)],'dialects':dialects,'keywords':keyword_rows,'exclusions':['feature examples','parser source bodies','generated parsers','test outputs','execution','mirror/graph wiring']}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: rules={len(rules)} tokens={len(tokens)} dialects={len(dialects)} keywords={len(keyword_rows)} bytes={out.stat().st_size}")
PY
