#!/usr/bin/env bash
# IANA Service Names and Port Numbers XML -> one pnix attrset chunk. redb 적재까지만.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-service-names-port-numbers"
MANIFEST="$SRC_DIR/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-service-names-port-numbers.generated.px"
CHUNK_SIZE=1500
CHUNK_INDEX=0
while [ $# -gt 0 ]; do
  case "$1" in
    --src-dir) SRC_DIR="$2"; shift 2;;
    --manifest) MANIFEST="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --chunk-size) CHUNK_SIZE="$2"; shift 2;;
    --chunk-index) CHUNK_INDEX="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC_DIR" "$MANIFEST" "$OUT" "$CHUNK_SIZE" "$CHUNK_INDEX" <<'PY'
import json, math, os, sys, xml.etree.ElementTree as ET
src_dir, manifest_path, out, chunk_size, chunk_index = sys.argv[1:]
chunk_size=int(chunk_size); chunk_index=int(chunk_index)
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
    return '{ name = "%s"; number = "%s"; protocol = "%s"; description = "%s"; assignee = "%s"; updated = "%s"; unauthorized = "%s"; sc = "%s"; xrefs = %s; }' % (
        esc(child_text(rec,'name')), esc(child_text(rec,'number')), esc(child_text(rec,'protocol')), esc(child_text(rec,'description')), esc(child_text(rec,'assignee')), esc(rec.get('updated') or ''), esc(child_text(rec,'unauthorized')), esc(child_text(rec,'sc')), list_lit(xrefs))
path=os.path.join(src_dir, manifest['source_path'])
root=ET.parse(path).getroot()
records=root.findall(ns+'record')
total=len(records)
chunk_count=(total + chunk_size - 1)//chunk_size if chunk_size > 0 else 1
if chunk_index < 0 or chunk_index >= chunk_count:
    raise SystemExit(f'chunk-index out of range: {chunk_index} / {chunk_count}')
start=chunk_index*chunk_size; end=min(total, start+chunk_size)
chunk_records=records[start:end]
lines=[]
lines.append('# stdlib/lib/corpus/iana-service-names-port-numbers.generated.px — GENERATED chunk, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-service-names-port-numbers.sh')
lines.append('{')
lines.append('  schema = "registry.iana.service_names_port_numbers.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','retrieved_at_utc','license','version_policy','source_url','source_path','source_sha256']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  registry = { id = "service-names-port-numbers"; title = "Service Name and Transport Protocol Port Number Registry"; records = [')
for rec in chunk_records:
    lines.append('    %s' % record_literal(rec))
lines.append('  ]; };')
lines.append('  registry_count = 1;')
lines.append('  record_count = %d;' % len(chunk_records))
lines.append('  total_record_count = %d;' % total)
lines.append('  chunk_index = %d;' % chunk_index)
lines.append('  chunk_count = %d;' % chunk_count)
lines.append('  chunk_size = %d;' % chunk_size)
lines.append('  generated_note = "IANA Service Names and Port Numbers XML structural transcription chunk only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: chunk={chunk_index+1}/{chunk_count} records={len(chunk_records)} total={total}')
PY
