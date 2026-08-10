#!/usr/bin/env bash
set -euo pipefail

# Convert gitignored containerd api/**/*.proto files into a pnix attrset source overlay.
# Host code is IO/transcription only. It stores original protobuf schema structure rows.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/code/containerd-api-proto/raw"
OUT="$ROOT/stdlib/lib/corpus/containerd-api-proto.generated.px"
RECEIPT="$ROOT/ingest/code/containerd-api-proto/source-receipt.json"
if [[ ! -d "$SRC" ]]; then
  echo "missing $SRC; run scripts/update-code-containerd-api-proto.sh first" >&2
  exit 1
fi

python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, re, sys
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
receipt=json.loads(receipt_path.read_text()) if receipt_path.exists() else {}

def esc(s):
    return json.dumps(s, ensure_ascii=False)
def arr(xs):
    return "[ " + " ".join(to_pnix(x) for x in xs) + " ]"
def obj(d):
    parts=[]
    for k,v in d.items():
        if v is None: continue
        parts.append(f'{k} = {to_pnix(v)};')
    return "{ " + " ".join(parts) + " }"
def to_pnix(v):
    if isinstance(v,bool): return "true" if v else "false"
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return arr(v)
    if isinstance(v,dict): return obj(v)
    return esc(str(v))

source_files=[]; packages=[]; imports=[]; messages=[]; enums=[]; enum_values=[]; fields=[]; services=[]; rpcs=[]
field_re=re.compile(r'^\s*(?:(optional|required|repeated)\s+)?([A-Za-z_][\w.<>]*(?:\s*<[^>]+>)?)\s+([A-Za-z_][\w]*)\s*=\s*([0-9]+)')
rpc_re=re.compile(r'^\s*rpc\s+([A-Za-z_][\w]*)\s*\(\s*([^)]*)\s*\)\s*returns\s*\(\s*([^)]*)\s*\)')
enum_value_re=re.compile(r'^\s*([A-Z][A-Z0-9_]+)\s*=\s*([0-9]+)')
container_stack=[]
for path in sorted(src.glob('*.proto')):
    rel=path.name.replace('__','/')
    text=path.read_text(errors='replace')
    lines=text.splitlines()
    pkg=None
    source_files.append({"path": rel, "bytes": path.stat().st_size, "lines": len(lines)})
    for i,line in enumerate(lines,1):
        line_no_comment=line.split('//',1)[0].strip()
        if not line_no_comment:
            continue
        m=re.match(r'^package\s+([^;]+);', line_no_comment)
        if m:
            pkg=m.group(1).strip(); packages.append({"file":rel,"name":pkg,"line":i}); continue
        m=re.match(r'^import\s+(?:public\s+|weak\s+)?"([^"]+)";', line_no_comment)
        if m:
            imports.append({"file":rel,"path":m.group(1),"line":i}); continue
        m=re.match(r'^message\s+([A-Za-z_][\w]*)\s*\{?', line_no_comment)
        if m:
            name=m.group(1); messages.append({"file":rel,"package":pkg or "","name":name,"line":i}); container_stack.append(("message",name)); continue
        m=re.match(r'^enum\s+([A-Za-z_][\w]*)\s*\{?', line_no_comment)
        if m:
            name=m.group(1); enums.append({"file":rel,"package":pkg or "","name":name,"line":i}); container_stack.append(("enum",name)); continue
        m=re.match(r'^service\s+([A-Za-z_][\w]*)\s*\{?', line_no_comment)
        if m:
            name=m.group(1); services.append({"file":rel,"package":pkg or "","name":name,"line":i}); container_stack.append(("service",name)); continue
        if line_no_comment.startswith('}') and container_stack:
            container_stack.pop(); continue
        container_type=container_stack[-1][0] if container_stack else ""
        container_name=container_stack[-1][1] if container_stack else ""
        m=field_re.match(line_no_comment)
        if m and container_type == "message":
            label,typ,name,num=m.groups()
            fields.append({"file":rel,"package":pkg or "","message":container_name,"label":label or "","type":typ.strip(),"name":name,"number":int(num),"line":i}); continue
        m=enum_value_re.match(line_no_comment)
        if m and container_type == "enum":
            name,num=m.groups(); enum_values.append({"file":rel,"package":pkg or "","enum":container_name,"name":name,"number":int(num),"line":i}); continue
        m=rpc_re.match(line_no_comment)
        if m and container_type == "service":
            name,req,res=m.groups(); rpcs.append({"file":rel,"package":pkg or "","service":container_name,"name":name,"request":req.strip(),"response":res.strip(),"line":i}); continue

data={
  "schema":"runtime.containerd.api_proto.v1",
  "source":"containerd/containerd api/**/*.proto",
  "license":"Apache-2.0",
  "ref":receipt.get("ref","unknown"),
  "archive_sha256":receipt.get("archive_sha256",""),
  "scope":"protobuf structural metadata only; no runtime state or execution",
  "source_files":source_files,
  "packages":packages,
  "imports":imports,
  "messages":messages,
  "enums":enums,
  "enum_values":enum_values,
  "fields":fields,
  "services":services,
  "rpcs":rpcs,
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+"\n")
print(f"generated {out}: files={len(source_files)} packages={len(packages)} messages={len(messages)} enums={len(enums)} enum_values={len(enum_values)} fields={len(fields)} services={len(services)} rpcs={len(rpcs)} bytes={out.stat().st_size}")
PY
