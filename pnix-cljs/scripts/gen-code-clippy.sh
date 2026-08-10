#!/usr/bin/env bash
# Clippy lint declarations -> pnix attrset source.
# Host script is IO/transcription only. It excludes lint implementations, docs prose, user lint results, configs, and autofix patches.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CLIPPY_SRC:-$ROOT/ingest/code/clippy/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/clippy.generated.px}"
RECEIPT="$ROOT/ingest/code/clippy/source-receipt.json"
if [[ ! -f "$SRC/clippy_lints/src/declared_lints.rs" || ! -f "$SRC/clippy_lints/src/deprecated_lints.rs" ]]; then
  echo "missing Clippy source under $SRC" >&2
  echo "run scripts/update-code-clippy.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, re, hashlib
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
root=src/'clippy_lints/src'
declared_p=root/'declared_lints.rs'
deprecated_p=root/'deprecated_lints.rs'
lib_p=root/'lib.rs'
license_apache=src/'LICENSE-APACHE'
license_mit=src/'LICENSE-MIT'

def read(p): return p.read_text(encoding='utf-8', errors='ignore')

def sha_text(t): return hashlib.sha256(t.encode('utf-8')).hexdigest()

def strip_clippy_prefix(s):
    return s[len('clippy::'):] if s.startswith('clippy::') else s

def lint_full(name):
    return 'clippy::'+name.lower()

def extract_blocks(text):
    lines=text.splitlines()
    blocks=[]; i=0
    while i < len(lines):
        if 'declare_clippy_lint!' in lines[i] and '{' in lines[i]:
            cur=[]; i+=1
            while i < len(lines):
                if lines[i].strip()=='}':
                    break
                cur.append(lines[i]); i+=1
            blocks.append('\n'.join(cur))
        i+=1
    return blocks

def parse_lint_block(block, rel):
    m=re.search(r'pub\s+([A-Z][A-Z0-9_]*)\s*,', block)
    if not m: return None
    name=m.group(1)
    after=block[m.end():].splitlines()
    cat=None
    for line in after:
        s=line.strip()
        if not s or s.startswith('//') or s.startswith('///') or s.startswith('#['):
            continue
        mm=re.match(r'([a-z_][a-z0-9_]*)\s*,', s)
        if mm:
            cat=mm.group(1); break
    if not cat: return None
    vm=re.search(r'#\[clippy::version\s*=\s*"([^"]*)"\]', block)
    return {
      'name':name,
      'full_name':lint_full(name),
      'category':cat,
      'version':vm.group(1) if vm else None,
      'source_file':rel,
    }

lints=[]; misses=[]
for p in sorted(root.rglob('*.rs')):
    if p.name in {'deprecated_lints.rs','declared_lints.rs'}: continue
    rel=str(p.relative_to(src))
    text=read(p)
    for block in extract_blocks(text):
        item=parse_lint_block(block, rel)
        if item is None: misses.append(rel)
        else: lints.append(item)
# Deterministic dedupe by name; keep first path-sorted declaration.
seen={}; dups=[]
for item in sorted(lints, key=lambda x:(x['name'],x['source_file'])):
    if item['name'] in seen: dups.append(item['name'])
    else: seen[item['name']]=item
lints=[seen[k] for k in sorted(seen)]

# deprecated_lints.rs stores pairs plus parallel version attributes. Store names/version only; reason prose excluded.
dep_text=read(deprecated_p)
def parse_versioned_pairs(label):
    mm=re.search(r'declare_with_version!\s*\{\s*'+label+r'\([^)]*\)\s*=\s*\[(.*?)\]\}', dep_text, re.S)
    if not mm: return []
    block=mm.group(1)
    out=[]
    pat=re.compile(r'#\[clippy::version\s*=\s*"([^"]*)"\]\s*\n\s*\("([^"]+)",\s*"([^"]*)"\)', re.S)
    for version,a,b in pat.findall(block):
        out.append({'version':version,'a':a,'b':b})
    return out
renamed=[{'from':x['a'], 'to':x['b'], 'version':x['version']} for x in parse_versioned_pairs('RENAMED')]
deprecated=[{'name':x['a'], 'version':x['version']} for x in parse_versioned_pairs('DEPRECATED')]
counts={}
for item in lints: counts[item['category']]=counts.get(item['category'],0)+1
categories=[{'category':k,'lint_count':counts[k]} for k in sorted(counts)]
# declared_lints.rs generated list count, for drift detection only.
declared_text=read(declared_p)
info_refs=re.findall(r'crate::[A-Za-z0-9_:]+_INFO', declared_text)
obj={
  'schema':'code.clippy.lints.v1',
  'source':{
    'name':'Clippy lint metadata',
    'repository':'rust-lang/rust-clippy',
    'license':'MIT OR Apache-2.0',
    'source_urls':['https://github.com/rust-lang/rust-clippy','https://github.com/rust-lang/rust-clippy/tree/master/clippy_lints/src'],
    'receipt':receipt,
    'generated_at':receipt.get('retrieved_at'),
    'generator':'scripts/gen-code-clippy.sh',
    'scope':'lint declaration/category/rename/deprecation metadata only; implementation bodies/docs prose/user lint results/configs/autofix patch payloads excluded'
  },
  'source_files':{
    'declared_lints_rs_sha256':sha_text(declared_text),
    'deprecated_lints_rs_sha256':sha_text(dep_text),
    'lib_rs_sha256':sha_text(read(lib_p)) if lib_p.exists() else None,
    'license_apache_sha256':sha_text(read(license_apache)) if license_apache.exists() else None,
    'license_mit_sha256':sha_text(read(license_mit)) if license_mit.exists() else None,
  },
  'summary':{
    'lint_count':len(lints),
    'category_count':len(categories),
    'renamed_count':len(renamed),
    'deprecated_count':len(deprecated),
    'declared_lints_info_ref_count':len(info_refs),
    'parse_miss_count':len(misses),
    'duplicate_name_count':len(dups),
    'lints_by_category':counts,
  },
  'categories':categories,
  'lints':sorted(lints, key=lambda x:(x['category'],x['name'])),
  'renamed':sorted(renamed, key=lambda x:(x['from'],x['to'])),
  'deprecated':sorted(deprecated, key=lambda x:x['name']),
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
content='# stdlib/lib/corpus/clippy.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-clippy.sh && scripts/gen-code-clippy.sh\n'
content+='# 범위: Clippy lint declaration/category/rename/deprecation metadata only. implementation/docs/lint results/configs/autofix payloads 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: lints={len(lints)} categories={len(categories)} renamed={len(renamed)} deprecated={len(deprecated)} misses={len(misses)} bytes={len(content.encode())}")
PY
