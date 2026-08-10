#!/usr/bin/env bash
# Conventional Commits spec -> pnix attrset source.
# Host script is IO/transcription only. It excludes prose body and any repository/user commit data.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CONVENTIONAL_COMMITS_SRC:-$ROOT/ingest/code/conventional-commits/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/conventional-commits.generated.px}"
RECEIPT="$ROOT/ingest/code/conventional-commits/source-receipt.json"
if [[ ! -d "$SRC" ]]; then
  echo "missing Conventional Commits source dir: $SRC" >&2
  echo "run scripts/update-code-conventional-commits.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, os, re, hashlib, datetime
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
spec=src/'content/v1.0.0/index.md'
text=spec.read_text(encoding='utf-8')
examples=re.findall(r'```\n(.*?)\n```', text, re.S)
header_template=examples[0].strip() if examples else '<type>[optional scope]: <description>\n\n[optional body]\n\n[optional footer(s)]'
recommended=sorted(set(re.findall(r'`(build|chore|ci|docs|style|refactor|perf|test|feat|fix|revert):?`', text)) | {'build','chore','ci','docs','style','refactor','perf','test','revert'})
mandated=['feat','fix']
semver=[
  {'trigger':'fix', 'bump':'PATCH', 'condition':'type == fix'},
  {'trigger':'feat', 'bump':'MINOR', 'condition':'type == feat'},
  {'trigger':'breaking_change', 'bump':'MAJOR', 'condition':'prefix bang or BREAKING CHANGE/BREAKING-CHANGE footer'},
]
structural_fields=[
  {'name':'type', 'required':True, 'position':'header.prefix', 'description':'noun token before optional scope'},
  {'name':'scope', 'required':False, 'position':'header.after_type', 'delimiter':'(...)'},
  {'name':'breaking_bang', 'required':False, 'position':'header.before_colon', 'token':'!'},
  {'name':'description', 'required':True, 'position':'header.after_colon_space'},
  {'name':'body', 'required':False, 'position':'after_blank_line'},
  {'name':'footer', 'required':False, 'position':'after_body_blank_line'},
]
footer_rules=[
  {'token_pattern':'word-token', 'separators':[': ',' #'], 'whitespace_rule':'use - in place of whitespace'},
  {'token':'BREAKING CHANGE', 'separator':': ', 'case':'uppercase'},
  {'token':'BREAKING-CHANGE', 'separator':': ', 'synonym_of':'BREAKING CHANGE'},
]
header_regex=r'^(?P<type>[A-Za-z0-9_-]+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?: (?P<description>.+)$'
license_text=(src/'LICENSE').read_text(encoding='utf-8') if (src/'LICENSE').exists() else ''
translations=sorted(str(p.relative_to(src)) for p in (src/'content/v1.0.0').glob('index.*.md'))
obj={
  'schema':'code.conventional_commits.v2',
  'source':{
    'name':'Conventional Commits specification',
    'repository':'conventional-commits/conventionalcommits.org',
    'license':'MIT',
    'source_urls':['https://www.conventionalcommits.org/','https://github.com/conventional-commits/conventionalcommits.org'],
    'receipt':receipt,
    'generated_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
    'generator':'scripts/gen-code-conventional-commits.sh',
    'scope':'commit-message syntax/structural metadata only; prose body, examples, repository commit logs, and user commit messages excluded'
  },
  'spec':{
    'version':'1.0.0',
    'source_path':'content/v1.0.0/index.md',
    'source_sha256':hashlib.sha256(text.encode()).hexdigest(),
    'license_sha256':hashlib.sha256(license_text.encode()).hexdigest(),
    'translation_file_count':len(translations),
    'translation_files':translations,
  },
  'grammar':{
    'header_template':header_template,
    'header_regex':header_regex,
    'structural_fields':structural_fields,
    'footer_rules':footer_rules,
    'breaking_change_markers':['!','BREAKING CHANGE','BREAKING-CHANGE'],
  },
  'types':{
    'mandated':mandated,
    'recommended_or_common':recommended,
    'open_world':True,
  },
  'semver_mapping':semver,
  'case_sensitivity':{'units_case_sensitive':False, 'breaking_change_token_uppercase':True},
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
content='# stdlib/lib/corpus/conventional-commits.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-conventional-commits.sh && scripts/gen-code-conventional-commits.sh\n'
content+='# 범위: Conventional Commits syntax/structural metadata only. 실제 commit log/user message/prose 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: mandated={len(mandated)} common_types={len(recommended)} translations={len(translations)} bytes={len(content.encode())}")
PY
