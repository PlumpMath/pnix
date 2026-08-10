#!/usr/bin/env bash
# IANA DNS Parameters XML -> pnix attrset source. 기관 XML registry columns 보존, redb 적재까지만.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-dns-parameters"
MANIFEST="$SRC_DIR/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-dns-parameters.generated.px"
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
def list_lit(xs): return '[ ' + ' '.join('"%s"' % esc(x) for x in xs if x) + ' ]'
def xref_text(x):
    vals=[]
    for k in ['type','data','section']:
        v=x.get(k) or ''
        if v: vals.append(k+'='+v)
    t=text(x)
    if t: vals.append('text='+t)
    return '|'.join(vals)
def field_text(c):
    tag=c.tag.split('}',1)[-1]
    if tag == 'xref': return ''
    attrs=''.join('|@%s=%s' % (k,v) for k,v in sorted(c.attrib.items()))
    return tag + '=' + text(c) + attrs
def record_literal(rec):
    fields=[field_text(c) for c in list(rec)]
    xrefs=[xref_text(x) for x in rec.findall(ns+'xref')]
    primary = child_text(rec,'value') or child_text(rec,'type') or child_text(rec,'name') or child_text(rec,'bit') or child_text(rec,'flag') or child_text(rec,'mnemonic')
    return '{ primary = "%s"; fields = %s; xrefs = %s; }' % (esc(primary), list_lit(fields), list_lit(xrefs))
path=os.path.join(src_dir, manifest['source_path'])
root=ET.parse(path).getroot()
registries=[]; total=0
for reg in root.findall(ns+'registry'):
    records=list(reg.findall(ns+'record'))
    total += len(records)
    registries.append({'id':reg.get('id') or '', 'title':child_text(reg,'title'), 'records':records})
lines=[]
lines.append('# stdlib/lib/corpus/iana-dns-parameters.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-dns-parameters.sh')
lines.append('{')
lines.append('  schema = "registry.iana.dns_parameters.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','retrieved_at_utc','license','version_policy','source_url','source_path','source_sha256']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  registries = [')
for reg in registries:
    lines.append('    { id = "%s"; title = "%s"; records = [' % (esc(reg['id']), esc(reg['title'])))
    for rec in reg['records']:
        lines.append('      %s' % record_literal(rec))
    lines.append('    ]; }')
lines.append('  ];')
lines.append('  registry_count = %d;' % len(registries))
lines.append('  record_count = %d;' % total)
lines.append('  generated_note = "IANA DNS Parameters XML structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: registries={len(registries)} records={total}')
PY
