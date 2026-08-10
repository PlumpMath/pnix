#!/usr/bin/env bash
# IANA TCP Parameters XML -> pnix attrset source. redb 적재까지만, graph/mirror/math 연결 없음.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-tcp-parameters"
MANIFEST="$SRC_DIR/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-tcp-parameters.generated.px"
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
def text(e): return ''.join(e.itertext()).strip() if e is not None else ''
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"').replace('\r',' ').replace('\n',' ').replace('\t',' ').replace('$','\\$')
def child_text(e,name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
def list_lit(xs): return '[ ' + ' '.join('"%s"' % esc(x) for x in xs if x) + ' ]'
def xref_text(x):
    vals=[]
    for k in ['type','data','section','registry','id']:
        v=x.get(k) or ''
        if v: vals.append(k+'='+v)
    t=text(x)
    if t: vals.append('text='+t)
    return '|'.join(vals)
def field_lit(name, value): return '{ name = "%s"; value = "%s"; }' % (esc(name), esc(value))
def record_literal(rec):
    fields=[]; xrefs=[]
    for c in list(rec):
        name=c.tag.split('}',1)[-1]
        if name == 'xref': xrefs.append(xref_text(c))
        else:
            v=text(c)
            if v: fields.append(field_lit(name, v))
    return '{ fields = [ %s ]; xrefs = %s; }' % (' '.join(fields), list_lit(xrefs))
path=os.path.join(src_dir, manifest['source_path'])
root=ET.parse(path).getroot()
registries=[]; total=0
for reg in root.findall(ns+'registry'):
    records=reg.findall(ns+'record')
    total += len(records)
    registries.append({'id':reg.get('id') or '', 'title':child_text(reg,'title'), 'records':records})
lines=[]
lines.append('# stdlib/lib/corpus/iana-tcp-parameters.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-tcp-parameters.sh')
lines.append('{')
lines.append('  schema = "registry.iana.tcp_parameters.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','retrieved_at_utc','license','version_policy','source_url','source_path','source_sha256']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  root_title = "%s";' % esc(child_text(root,'title')))
lines.append('  registries = [')
for reg in registries:
    lines.append('    { id = "%s"; title = "%s"; records = [' % (esc(reg['id']), esc(reg['title'])))
    for rec in reg['records']:
        lines.append('      %s' % record_literal(rec))
    lines.append('    ]; }')
lines.append('  ];')
lines.append('  registry_count = %d;' % len(registries))
lines.append('  record_count = %d;' % total)
lines.append('  generated_note = "IANA TCP Parameters XML structural transcription only; XML child tag names preserved as field name/value pairs; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: registries={len(registries)} records={total}')
PY
