#!/usr/bin/env bash
# Ruff rule registry -> pnix attrset source.
# Host script is IO/transcription only. It excludes rule implementations, docs prose, lint results, configs, and autofix patches.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${RUFF_SRC:-$ROOT/ingest/code/ruff/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/ruff.generated.px}"
RECEIPT="$ROOT/ingest/code/ruff/source-receipt.json"
if [[ ! -f "$SRC/crates/ruff_linter/src/codes.rs" ]]; then
  echo "missing Ruff codes.rs under $SRC" >&2
  echo "run scripts/update-code-ruff.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, re, hashlib
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
codes_p=src/'crates/ruff_linter/src/codes.rs'
registry_p=src/'crates/ruff_linter/src/registry.rs'
redirects_p=src/'crates/ruff_linter/src/rule_redirects.rs'
license_p=src/'LICENSE'
codes=codes_p.read_text(encoding='utf-8')
registry=registry_p.read_text(encoding='utf-8')
redirects=redirects_p.read_text(encoding='utf-8')
license_text=license_p.read_text(encoding='utf-8') if license_p.exists() else ''
# Parse Linter enum variants with doc url/name and one or more prefixes.
linters={}
block=re.search(r'pub enum Linter \{(.*?)\n\}', registry, re.S).group(1)
comments=[]; prefixes=[]
for line in block.splitlines():
    s=line.strip()
    if s.startswith('///'):
        comments.append(s[3:].strip())
    elif s.startswith('#[prefix'):
        m=re.search(r'"([^"]+)"', s)
        if m: prefixes.append(m.group(1))
    else:
        m=re.match(r'([A-Za-z][A-Za-z0-9_]*),', s)
        if m:
            variant=m.group(1)
            doc=' '.join(comments).strip()
            url=None; name=doc
            md=re.search(r'\[([^\]]+)\]\(([^)]+)\)', doc)
            if md:
                name=md.group(1); url=md.group(2)
            linters[variant]={'id':variant,'name':name,'url':url,'prefixes':prefixes[:]}
            comments=[]; prefixes=[]
# Parse rule code mappings; skip cfg test rules.
rules=[]
code_re=re.compile(r'\((\w+),\s*"([^"]+)"\)\s*=>\s*rules::([^,]+),')
prev_cfg=False
for line in codes.splitlines():
    s=line.strip()
    if s.startswith('#[cfg('):
        prev_cfg=True
        continue
    m=code_re.search(s)
    if m:
        if prev_cfg:
            prev_cfg=False
            continue
        linter,suffix,path=m.groups()
        prefixes=linters.get(linter,{}).get('prefixes',[])
        if any(suffix.startswith(p) for p in prefixes):
            code=suffix
        elif prefixes:
            code=prefixes[0]+suffix
        else:
            code=suffix
        variant=path.split('::')[-1]
        module='::'.join(path.split('::')[:-1])
        rules.append({'code':code,'suffix':suffix,'linter':linter,'linter_prefixes':prefixes,'variant':variant,'module_path':module})
        prev_cfg=False
        continue
    if s and not s.startswith('//'):
        prev_cfg=False
# Parse redirects only string pairs.
redirect_list=[]
for a,b in re.findall(r'\("([A-Za-z0-9]+)",\s*"([A-Za-z0-9]+)"\)', redirects):
    redirect_list.append({'from':a,'to':b})
counts={}
for r in rules:
    counts[r['linter']]=counts.get(r['linter'],0)+1
obj={
  'schema':'code.ruff.rules.v1',
  'source':{
    'name':'Ruff rule metadata',
    'repository':'astral-sh/ruff',
    'license':'MIT',
    'source_urls':['https://docs.astral.sh/ruff/','https://github.com/astral-sh/ruff'],
    'receipt':receipt,
    'generated_at':receipt.get('retrieved_at'),
    'generator':'scripts/gen-code-ruff.sh',
    'scope':'rule code/linter/redirect metadata only; rule implementations/docs prose/user lint results/configs/autofix patch payloads excluded'
  },
  'source_files':{
    'codes_rs_sha256':hashlib.sha256(codes.encode()).hexdigest(),
    'registry_rs_sha256':hashlib.sha256(registry.encode()).hexdigest(),
    'rule_redirects_rs_sha256':hashlib.sha256(redirects.encode()).hexdigest(),
    'license_sha256':hashlib.sha256(license_text.encode()).hexdigest() if license_text else None,
  },
  'summary':{
    'rule_count':len(rules),
    'linter_count':len(linters),
    'redirect_count':len(redirect_list),
    'rules_by_linter':counts,
  },
  'linters':[linters[k] for k in sorted(linters)],
  'rules':sorted(rules, key=lambda r:(r['linter'],r['code'],r['variant'])),
  'redirects':sorted(redirect_list, key=lambda r:(r['from'],r['to'])),
}
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
content='# stdlib/lib/corpus/ruff.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-ruff.sh && scripts/gen-code-ruff.sh\n'
content+='# 범위: Ruff rule code/linter/redirect metadata only. implementations/docs/lint results/configs/autofix payloads 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: rules={len(rules)} linters={len(linters)} redirects={len(redirect_list)} bytes={len(content.encode())}")
PY
