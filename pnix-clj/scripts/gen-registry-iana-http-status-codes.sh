#!/usr/bin/env bash
# IANA HTTP Status Code Registry XML -> pnix attrset source. redb 적재까지만.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-http-status-codes"
MANIFEST="$SRC_DIR/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-http-status-codes.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --src-dir) SRC_DIR="$2"; shift 2;;
    --manifest) MANIFEST="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC_DIR" "$MANIFEST" "$OUT" <<'PY'
import json, os, sys, xml.etree.ElementTree as ET
src_dir, manifest_path, out = sys.argv[1:]
manifest=json.load(open(manifest_path,encoding='utf-8'))
ns='{http://www.iana.org/assignments}'
def text(e): return ''.join(e.itertext()).strip()
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"').replace('\r',' ').replace('\n',' ').replace('\t',' ').replace('$','\\$')
def child_text(e,name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
def xref_text(x):
    vals=[]
    for k in ['type','data','section']:
        v=x.get(k) or ''
        if v: vals.append(k+'='+v)
    t=text(x)
    if t: vals.append('text='+t)
    return '|'.join(vals)
def list_lit(xs): return '[ ' + ' '.join('"%s"' % esc(x) for x in xs if x) + ' ]'
def record_literal(rec):
    xrefs=[xref_text(x) for x in rec.findall(ns+'xref')]
    return '{ value = "%s"; description = "%s"; xrefs = %s; }' % (esc(child_text(rec,'value')), esc(child_text(rec,'description')), list_lit(xrefs))
path=os.path.join(src_dir, manifest['source_path'])
root=ET.parse(path).getroot()
reg=root.find(ns+'registry')
records=reg.findall(ns+'record') if reg is not None else []
lines=[]
lines.append('# stdlib/lib/corpus/iana-http-status-codes.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-http-status-codes.sh')
lines.append('{')
lines.append('  schema = "registry.iana.http_status_codes.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','retrieved_at_utc','license','version_policy','source_url','source_path','source_sha256']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  registry = { id = "http-status-codes-1"; title = "HTTP Status Codes"; records = [')
for rec in records:
    lines.append('    %s' % record_literal(rec))
lines.append('  ]; };')
lines.append('  registry_count = 1;')
lines.append('  record_count = %d;' % len(records))
lines.append('  generated_note = "IANA HTTP Status Code XML structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: records={len(records)}')
PY
