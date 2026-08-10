#!/usr/bin/env bash
# OPA/Rego capabilities -> pnix attrset source.
# Host script is IO/transcription only. No policy semantics are interpreted here.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OPA_BIN="${OPA_BIN:-$ROOT/ingest/code/opa/opa-current}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/opa-rego.generated.px}"
if [[ ! -x "$OPA_BIN" ]]; then
  echo "missing executable OPA binary: $OPA_BIN" >&2
  echo "run scripts/update-code-opa-rego.sh first" >&2
  exit 2
fi
python3 - "$OPA_BIN" "$OUT" "$ROOT/ingest/code/opa/source-receipt.json" <<'PY'
import json, subprocess, sys, datetime, os, hashlib
opa,out,receipt_path=sys.argv[1:]

def run(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)

def try_json(args):
    try:
        s=run(args)
        return json.loads(s), args, s
    except Exception:
        return None, args, None

version,_,version_text = try_json([opa,'version','--format=json'])
if version is None:
    version_text=run([opa,'version'])
    version={'raw':version_text}

cap=None; cap_cmd=None; cap_raw=None
for args in ([opa,'capabilities','--current'], [opa,'capabilities']):
    cap,cap_cmd,cap_raw=try_json(args)
    if cap is not None:
        break
if cap is None:
    cap={'error':'opa capabilities command did not return json'}
    cap_cmd=[]; cap_raw=''

try:
    receipt=json.load(open(receipt_path))
except Exception:
    receipt={}

keywords=[
  'package','import','default','else','not','some','with','as','if','contains','in','every',
  'true','false','null'
]
operators=[
  {'symbol':':=','name':'assignment'}, {'symbol':'=','name':'unification'}, {'symbol':'==','name':'equality'},
  {'symbol':'!=','name':'inequality'}, {'symbol':'<','name':'less_than'}, {'symbol':'<=','name':'less_or_equal'},
  {'symbol':'>','name':'greater_than'}, {'symbol':'>=','name':'greater_or_equal'}, {'symbol':'+','name':'plus'},
  {'symbol':'-','name':'minus'}, {'symbol':'*','name':'multiply'}, {'symbol':'/','name':'divide'},
  {'symbol':'%','name':'modulo'}, {'symbol':'&','name':'set_intersection'}, {'symbol':'|','name':'set_union'},
  {'symbol':'[ ]','name':'array_index'}, {'symbol':'.','name':'reference'}, {'symbol':'{ }','name':'object_or_set'},
  {'symbol':'( )','name':'grouping_or_call'}
]

builtins=cap.get('builtins',[]) if isinstance(cap,dict) else []
obj={
  'schema':'code.opa_rego.v1',
  'source':{
    'name':'Open Policy Agent / Rego',
    'license':'Apache-2.0',
    'source_urls':['https://www.openpolicyagent.org/','https://github.com/open-policy-agent/opa'],
    'receipt':receipt,
    'generated_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
    'generator':'scripts/gen-code-opa-rego.sh',
    'scope':'OPA capabilities/builtin metadata + Rego keyword/operator tokens only; no policy modules or authorization decisions'
  },
  'opa_version':version,
  'capabilities_command':' '.join(cap_cmd) if cap_cmd else None,
  'capabilities_sha256':hashlib.sha256((cap_raw or '').encode()).hexdigest(),
  'builtin_count':len(builtins),
  'keyword_count':len(keywords),
  'operator_count':len(operators),
  'keywords':keywords,
  'operators':operators,
  'capabilities':cap
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
        items=[]
        for k in sorted(v.keys()):
            items.append(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';')
        return '{\n' + '\n'.join(items) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)

text='# stdlib/lib/corpus/opa-rego.generated.px — GENERATED, do not commit.\n'
text+='# 생성: scripts/update-code-opa-rego.sh && scripts/gen-code-opa-rego.sh\n'
text+='# 범위: OPA/Rego capabilities/builtins/keywords/operators metadata only. 정책/권한판단 데이터 제외.\n'
text+=pnix(obj)+'\n'
os.makedirs(os.path.dirname(out), exist_ok=True)
open(out,'w',encoding='utf-8').write(text)
print(f"generated {out}: builtins={len(builtins)} keywords={len(keywords)} operators={len(operators)} bytes={len(text.encode())}")
PY
