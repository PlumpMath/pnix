#!/usr/bin/env bash
# IANA Syslog Parameters XML -> pnix attrset source. 기관 XML registry schema 보존, redb 적재까지만.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-syslog-parameters"
MANIFEST="$SRC_DIR/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-syslog-parameters.generated.px"
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
def text(e): return (e.text or '').strip()
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"').replace('\r',' ').replace('\n',' ').replace('\t',' ').replace('$','\\$')
def child_text(e,name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
def emit_xref(x): return '{ type = "%s"; data = "%s"; section = "%s"; }' % (esc(x.get('type') or ''), esc(x.get('data') or ''), esc(x.get('section') or ''))
def emit_record(r):
    return '{ id = "%s"; value = "%s"; version = "%s"; status = "%s"; description = "%s"; xrefs = [ %s ]; }' % (
      esc(r['id']), esc(r['value']), esc(r['version']), esc(r['status']), esc(r['description']), ' '.join(emit_xref(x) for x in r['xrefs']))
path=os.path.join(src_dir, manifest['source_path'])
root=ET.parse(path).getroot()
registries=[]; total=0
for reg in root.findall(ns+'registry'):
    records=[]
    for rec in reg.findall(ns+'record'):
        xrefs=[dict(x.attrib) for x in rec.findall(ns+'xref')]
        records.append({
            'id': child_text(rec,'id'),
            'value': child_text(rec,'value'),
            'version': child_text(rec,'version'),
            'status': child_text(rec,'status'),
            'description': child_text(rec,'description'),
            'xrefs': xrefs,
        })
    total += len(records)
    registries.append({'id':reg.get('id') or '', 'title':child_text(reg,'title'), 'records':records})
lines=[]
lines.append('# stdlib/lib/corpus/iana-syslog-parameters.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-syslog-parameters.sh')
lines.append('{')
lines.append('  schema = "registry.iana.syslog_parameters.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','retrieved_at_utc','license','version_policy','source_url','source_path','source_sha256']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  registries = [')
for reg in registries:
    lines.append('    { id = "%s"; title = "%s"; records = [' % (esc(reg['id']), esc(reg['title'])))
    for r in reg['records']:
        lines.append('      %s' % emit_record(r))
    lines.append('    ]; }')
lines.append('  ];')
lines.append('  registry_count = %d;' % len(registries))
lines.append('  record_count = %d;' % total)
lines.append('  generated_note = "IANA Syslog Parameters XML structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: registries={len(registries)} records={total}')
PY
