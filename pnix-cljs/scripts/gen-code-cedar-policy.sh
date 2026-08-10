#!/usr/bin/env bash
# Cedar grammar/schema metadata -> pnix attrset source.
# Host script is IO/transcription only. No policy semantics, no policy evaluation.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CEDAR_SRC:-$ROOT/ingest/code/cedar/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/cedar-policy.generated.px}"
RECEIPT="$ROOT/ingest/code/cedar/source-receipt.json"
if [[ ! -d "$SRC" ]]; then
  echo "missing Cedar source dir: $SRC" >&2
  echo "run scripts/update-code-cedar-policy.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, os, re, hashlib, datetime
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try:
    import tomllib
except Exception:
    tomllib=None
try:
    receipt=json.load(open(receipt_path))
except Exception:
    receipt={}

def read_rel(rel):
    p=src/rel
    text=p.read_text(encoding='utf-8')
    return {
        'path':rel,
        'bytes':len(text.encode()),
        'lines':text.count('\n')+1,
        'sha256':hashlib.sha256(text.encode()).hexdigest(),
        'content':text,
    }

def match_block(text):
    m=re.search(r'\nmatch\s*\{', text)
    if not m: return ''
    start=m.end(); depth=1; i=start
    while i < len(text):
        c=text[i]
        if c=='{': depth+=1
        elif c=='}':
            depth-=1
            if depth==0: return text[start:i]
        i+=1
    return text[start:]

def grammar_meta(rel, name):
    f=read_rel(rel); text=f['content']; mb=match_block(text)
    string_tokens=sorted(set(re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"\s*(?:=>\s*[A-Z_][A-Z0-9_]*)?', mb)))
    regex_tokens=sorted(set(re.findall(r'r#?"([^\n"]+)"#?', mb)))
    keywords=sorted(t for t in string_tokens if re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*', t))
    operators=sorted(t for t in string_tokens if not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*', t))
    rules=[]
    for line in text.splitlines():
        mm=re.match(r'^(pub\s+)?([A-Za-z_][A-Za-z0-9_]*)(?:<[^>]+>)?\s*:', line)
        if mm:
            rules.append({'name':mm.group(2), 'public':bool(mm.group(1))})
    return {
        'name':name,
        'file':f,
        'rule_count':len(rules),
        'rules':rules,
        'keyword_count':len(keywords),
        'keywords':keywords,
        'operator_count':len(operators),
        'operators':operators,
        'regex_token_count':len(regex_tokens),
        'regex_tokens':regex_tokens,
    }

def cargo_workspace():
    p=src/'Cargo.toml'
    raw=p.read_text(encoding='utf-8')
    packages=[]; version=None; rust_version=None; license_id=None; repo=None
    if tomllib:
        data=tomllib.loads(raw)
        packages=data.get('workspace',{}).get('members',[])
        wp=data.get('workspace',{}).get('package',{})
        version=wp.get('version'); rust_version=wp.get('rust-version'); license_id=wp.get('license'); repo=wp.get('repository')
    else:
        version=re.search(r'^version\s*=\s*"([^"]+)"', raw, re.M)
        version=version.group(1) if version else None
        packages=re.findall(r'"([^"]+)"', re.search(r'members\s*=\s*\[(.*?)\]', raw, re.S).group(1))
    return {
        'path':'Cargo.toml',
        'sha256':hashlib.sha256(raw.encode()).hexdigest(),
        'workspace_members':packages,
        'workspace_member_count':len(packages),
        'version':version,
        'rust_version':rust_version,
        'license':license_id,
        'repository':repo,
    }

extension_dirs=[
    'cedar-policy-core/src/extensions',
    'cedar-policy-core/src/validator/extensions',
]
extensions=[]
for d in extension_dirs:
    pp=src/d
    if pp.is_dir():
        for f in sorted(pp.glob('*.rs')):
            if f.name=='mod.rs': continue
            text=f.read_text(encoding='utf-8')
            extensions.append({'module':f.stem, 'path':str(f.relative_to(src)), 'sha256':hashlib.sha256(text.encode()).hexdigest(), 'bytes':len(text.encode())})

obj={
    'schema':'code.cedar_policy.language.v1',
    'source':{
        'name':'Cedar Policy Language',
        'repository':'cedar-policy/cedar',
        'license':'Apache-2.0',
        'source_urls':['https://www.cedarpolicy.com/','https://github.com/cedar-policy/cedar'],
        'receipt':receipt,
        'generated_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
        'generator':'scripts/gen-code-cedar-policy.sh',
        'scope':'language grammar/schema metadata only; no actual policies, authorization decisions, decision logs, or bypass procedures'
    },
    'workspace':cargo_workspace(),
    'grammars':[
        grammar_meta('cedar-policy-core/src/parser/grammar.lalrpop','cedar_policy_language'),
        grammar_meta('cedar-policy-core/src/validator/cedar_schema/grammar.lalrpop','cedar_schema_language'),
    ],
    'extensions':extensions,
    'extension_count':len(extensions),
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
text='# stdlib/lib/corpus/cedar-policy.generated.px — GENERATED, do not commit.\n'
text+='# 생성: scripts/update-code-cedar-policy.sh && scripts/gen-code-cedar-policy.sh\n'
text+='# 범위: Cedar language/schema grammar metadata. 실제 정책/권한판단/bypass 제외.\n'
text+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(text, encoding='utf-8')
print(f"generated {out}: grammars={len(obj['grammars'])} extensions={len(extensions)} bytes={len(text.encode())}")
PY
