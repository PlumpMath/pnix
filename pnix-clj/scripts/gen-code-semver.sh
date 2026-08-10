#!/usr/bin/env bash
# Semantic Versioning spec -> pnix attrset source.
# Host script is IO/transcription only. It excludes prose body and package/version datasets.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${SEMVER_SRC:-$ROOT/ingest/code/semver/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/semver.generated.px}"
RECEIPT="$ROOT/ingest/code/semver/source-receipt.json"
if [[ ! -f "$SRC/semver.md" ]]; then
  echo "missing semver.md under $SRC" >&2
  echo "run scripts/update-code-semver.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, re, hashlib, datetime
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
text=(src/'semver.md').read_text(encoding='utf-8')
ver=re.search(r'Semantic Versioning\s+([0-9]+\.[0-9]+\.[0-9]+)', text)
spec_version=ver.group(1) if ver else '2.0.0'
license_id='CC-BY-3.0' if 'CC BY 3.0' in text else 'unknown'
obj={
  'schema':'code.semver.v1',
  'source':{
    'name':'Semantic Versioning specification',
    'repository':'semver/semver',
    'license':license_id,
    'source_urls':['https://semver.org/','https://github.com/semver/semver'],
    'receipt':receipt,
    'generated_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
    'generator':'scripts/gen-code-semver.sh',
    'scope':'version grammar/precedence metadata only; prose body, package registries, release histories, and user constraints excluded'
  },
  'spec':{
    'version':spec_version,
    'source_path':'semver.md',
    'source_sha256':hashlib.sha256(text.encode()).hexdigest(),
    'license':'CC-BY-3.0',
  },
  'version_core':{
    'format':'MAJOR.MINOR.PATCH',
    'components':[
      {'name':'major','type':'non_negative_integer','leading_zeroes':False},
      {'name':'minor','type':'non_negative_integer','leading_zeroes':False},
      {'name':'patch','type':'non_negative_integer','leading_zeroes':False},
    ],
    'regex':'^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(?:-((?:0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)(?:\\.(?:0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\\+([0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*))?$',
  },
  'pre_release':{
    'prefix':'-',
    'separator':'.',
    'identifier_charset':'ASCII alphanumeric plus hyphen',
    'empty_identifier_allowed':False,
    'numeric_leading_zeroes_allowed':False,
    'normal_version_precedence':'lower_than_associated_normal_version',
  },
  'build_metadata':{
    'prefix':'+',
    'separator':'.',
    'identifier_charset':'ASCII alphanumeric plus hyphen',
    'empty_identifier_allowed':False,
    'affects_precedence':False,
  },
  'precedence_rules':[
    {'order':1,'compare':'major','method':'numeric'},
    {'order':2,'compare':'minor','method':'numeric'},
    {'order':3,'compare':'patch','method':'numeric'},
    {'order':4,'compare':'pre_release_presence','method':'normal_version_greater_than_pre_release'},
    {'order':5,'compare':'pre_release_identifiers','method':'left_to_right'},
    {'order':6,'compare':'numeric_identifier','method':'numeric'},
    {'order':7,'compare':'non_numeric_identifier','method':'ascii_lexical'},
    {'order':8,'compare':'numeric_vs_non_numeric','method':'numeric_lower_than_non_numeric'},
    {'order':9,'compare':'identifier_count','method':'larger_set_higher_if_all_previous_equal'},
    {'order':10,'compare':'build_metadata','method':'ignored'},
  ],
  'increment_mapping':[
    {'component':'major','trigger':'backwards_incompatible_public_api_change','reset':['minor','patch']},
    {'component':'minor','trigger':'backwards_compatible_public_api_functionality_or_deprecation','reset':['patch']},
    {'component':'patch','trigger':'backwards_compatible_bug_fix','reset':[]},
  ],
  'special_versions':[
    {'pattern':'0.y.z','meaning':'initial_development','stable_public_api':False},
    {'version':'1.0.0','meaning':'public_api_defined'},
  ],
  'precedence_example_chain':['1.0.0-alpha','1.0.0-alpha.1','1.0.0-alpha.beta','1.0.0-beta','1.0.0-beta.2','1.0.0-beta.11','1.0.0-rc.1','1.0.0'],
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
content='# stdlib/lib/corpus/semver.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-semver.sh && scripts/gen-code-semver.sh\n'
content+='# 범위: Semantic Versioning grammar/precedence metadata only. prose/package registry/release history 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: version={spec_version} precedence_rules={len(obj['precedence_rules'])} bytes={len(content.encode())}")
PY
