#!/usr/bin/env bash
# SARIF 2.1.0 JSON schema -> pnix attrset source.
# Host script is IO/transcription only. Prose fields and actual SARIF logs are excluded.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${SARIF_SCHEMA:-$ROOT/ingest/code/sarif/sarif-schema-2.1.0.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/sarif.generated.px}"
RECEIPT="$ROOT/ingest/code/sarif/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing SARIF schema: $SRC" >&2
  echo "run scripts/update-code-sarif.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, hashlib
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
raw=json.load(open(src))
PROSE={'description','markdownDescription','examples','example','$comment','comments'}
removed={k:0 for k in PROSE}
def strip(v):
    if isinstance(v, dict):
        out={}
        for k,val in v.items():
            if k in PROSE:
                removed[k]=removed.get(k,0)+1
                continue
            out[k]=strip(val)
        return out
    if isinstance(v, list):
        return [strip(x) for x in v]
    return v
san=strip(raw)
defs=san.get('definitions',{})
properties=san.get('properties',{}) or {}
def_summary=[]
enums=[]
required=[]
prop_paths=[]
def walk(node,path):
    if isinstance(node,dict):
        if 'enum' in node and isinstance(node['enum'],list):
            enums.append({'path':'.'.join(path), 'values':node['enum'], 'count':len(node['enum'])})
        if 'required' in node and isinstance(node['required'],list):
            required.append({'path':'.'.join(path), 'fields':node['required'], 'count':len(node['required'])})
        props=node.get('properties')
        if isinstance(props,dict):
            for k,val in sorted(props.items()):
                prop_paths.append({'path':'.'.join(path+[k]), 'keys':sorted(val.keys()) if isinstance(val,dict) else []})
                walk(val,path+[k])
        for k,val in node.items():
            if k not in ('properties',):
                walk(val,path+[k])
    elif isinstance(node,list):
        for i,x in enumerate(node): walk(x,path+[str(i)])
for name,node in sorted(defs.items()):
    props=(node.get('properties') or {}) if isinstance(node,dict) else {}
    req=node.get('required',[]) if isinstance(node,dict) else []
    def_summary.append({'name':name, 'type':node.get('type') if isinstance(node,dict) else None, 'property_count':len(props), 'required':req})
walk(san,[])
interesting_defs=[x for x in def_summary if x['name'] in ['run','result','reportingDescriptor','physicalLocation','artifactLocation','fix','replacement','threadFlowLocation','level','message','tool','artifact']]
obj={
  'schema':'code.sarif.schema.v1',
  'source':{
    'name':'SARIF 2.1.0 JSON Schema',
    'license':'OASIS-Specification',
    'source_urls':['https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json','https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/sarif-v2.1.0-os.html'],
    'receipt':receipt,
    'generated_at':receipt.get('retrieved_at'),
    'generator':'scripts/gen-code-sarif.sh',
    'scope':'SARIF JSON schema structure only; prose description/example fields, actual logs, source locations, and autofix payloads excluded'
  },
  'official_json_schema':{
    'title':raw.get('title'),
    'draft':raw.get('$schema'),
    'source_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),
    'prose_fields_removed':removed,
    'sanitized_schema':san,
  },
  'summary':{
    'definition_count':len(defs),
    'top_property_count':len(properties),
    'enum_count':len(enums),
    'required_block_count':len(required),
    'property_path_count':len(prop_paths),
    'interesting_definitions':interesting_defs,
  },
  'definitions':def_summary,
  'enums':enums,
  'required_blocks':required,
  'property_paths':prop_paths,
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
content='# stdlib/lib/corpus/sarif.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-sarif.sh && scripts/gen-code-sarif.sh\n'
content+='# 범위: SARIF JSON schema structure only. descriptions/examples/logs/source locations/autofix payloads 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: defs={len(defs)} enums={len(enums)} required={len(required)} props={len(prop_paths)} bytes={len(content.encode())}")
PY
