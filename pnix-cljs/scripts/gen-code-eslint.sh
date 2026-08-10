#!/usr/bin/env bash
# ESLint core rule metadata -> pnix attrset source.
# Host script is IO/transcription only. It excludes prose docs, implementations, lint results, configs, and autofix patches.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ESLINT_SRC:-$ROOT/ingest/code/eslint/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/eslint.generated.px}"
RECEIPT="$ROOT/ingest/code/eslint/source-receipt.json"
if [[ ! -f "$SRC/docs/src/_data/rules_meta.json" ]]; then
  echo "missing ESLint rules_meta.json under $SRC" >&2
  echo "run scripts/update-code-eslint.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, hashlib, re
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
meta_path=src/'docs/src/_data/rules_meta.json'
rules_path=src/'docs/src/_data/rules.json'
cat_path=src/'docs/src/_data/rules_categories.js'
license_path=src/'LICENSE'
meta=json.load(open(meta_path))
rules_json=json.load(open(rules_path)) if rules_path.exists() else {}
cat_text=cat_path.read_text(encoding='utf-8') if cat_path.exists() else ''
license_text=license_path.read_text(encoding='utf-8') if license_path.exists() else ''
category_display={}
for m in re.finditer(r'\n\s*(problem|suggestion|layout|deprecated|removed):\s*\{\s*displayName:\s*"([^"]+)"', cat_text, re.S):
    category_display[m.group(1)]=m.group(2)
for k,v in {'problem':'Possible Problems','suggestion':'Suggestions','layout':'Layout & Formatting','deprecated':'Deprecated','removed':'Removed'}.items():
    category_display.setdefault(k,v)
removed_map={}
for item in rules_json.get('removed',[]) if isinstance(rules_json,dict) else []:
    reps=[]
    for r in item.get('replacedBy',[]) or []:
        rr=r.get('rule',{}) if isinstance(r,dict) else {}
        if rr.get('name'):
            reps.append({'rule':rr.get('name'), 'url':rr.get('url')})
    removed_map[item.get('removed')]={'name':item.get('removed'), 'replaced_by':reps}

def replacement_slim(dep):
    reps=[]
    if not isinstance(dep,dict): return reps
    for r in dep.get('replacedBy',[]) or []:
        if not isinstance(r,dict): continue
        rr=r.get('rule',{}) or {}
        pp=r.get('plugin',{}) or {}
        reps.append({
            'rule':rr.get('name'),
            'rule_url':rr.get('url'),
            'plugin':pp.get('name'),
            'plugin_url':pp.get('url'),
            'url':r.get('url'),
        })
    return reps
rules=[]
for name,m in sorted(meta.items()):
    docs=m.get('docs',{}) if isinstance(m,dict) else {}
    dep=m.get('deprecated') if isinstance(m,dict) else None
    rules.append({
        'id':name,
        'category':m.get('type') if isinstance(m,dict) else None,
        'recommended':bool(docs.get('recommended')),
        'docs_url':docs.get('url'),
        'fixable':m.get('fixable') if isinstance(m,dict) else None,
        'has_suggestions':bool(m.get('hasSuggestions')) if isinstance(m,dict) else False,
        'deprecated':dep is not None,
        'deprecated_since':dep.get('deprecatedSince') if isinstance(dep,dict) else None,
        'available_until':dep.get('availableUntil') if isinstance(dep,dict) else None,
        'replaced_by':replacement_slim(dep),
    })
counts={}
for r in rules:
    counts[r['category']]=counts.get(r['category'],0)+1
summary={
    'rule_count':len(rules),
    'category_counts':counts,
    'recommended_count':sum(1 for r in rules if r['recommended']),
    'fixable_count':sum(1 for r in rules if r['fixable']),
    'suggestion_count':sum(1 for r in rules if r['has_suggestions']),
    'deprecated_count':sum(1 for r in rules if r['deprecated']),
    'removed_count':len(removed_map),
}
obj={
  'schema':'code.eslint.rules.v1',
  'source':{
    'name':'ESLint core rule metadata',
    'repository':'eslint/eslint',
    'license':'MIT',
    'source_urls':['https://eslint.org/','https://github.com/eslint/eslint'],
    'receipt':receipt,
    'generated_at':receipt.get('retrieved_at'),
    'generator':'scripts/gen-code-eslint.sh',
    'scope':'core rule metadata only; rule docs prose/implementation source/user lint results/configs/autofix patch payloads excluded'
  },
  'source_files':{
    'rules_meta_json_sha256':hashlib.sha256(meta_path.read_bytes()).hexdigest(),
    'rules_json_sha256':hashlib.sha256(rules_path.read_bytes()).hexdigest() if rules_path.exists() else None,
    'categories_js_sha256':hashlib.sha256(cat_text.encode()).hexdigest() if cat_text else None,
    'license_sha256':hashlib.sha256(license_text.encode()).hexdigest() if license_text else None,
  },
  'categories':[{'id':k,'display_name':category_display[k]} for k in sorted(category_display)],
  'summary':summary,
  'rules':rules,
  'removed_rules':[removed_map[k] for k in sorted(removed_map)],
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
content='# stdlib/lib/corpus/eslint.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-eslint.sh && scripts/gen-code-eslint.sh\n'
content+='# 범위: ESLint core rule metadata only. docs prose/implementation/lint results/configs/autofix payloads 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: rules={len(rules)} deprecated={summary['deprecated_count']} removed={summary['removed_count']} bytes={len(content.encode())}")
PY
