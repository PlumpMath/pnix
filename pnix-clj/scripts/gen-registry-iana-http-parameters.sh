#!/usr/bin/env bash
# IANA HTTP Parameters XML -> pnix attrset source. 기관 XML field schema를 보존한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/registry/iana-http-parameters/http-parameters.xml"
MANIFEST="$ROOT/ingest/registry/iana-http-parameters/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-http-parameters.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --manifest) MANIFEST="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$MANIFEST" "$OUT" <<'PY'
import json, sys, xml.etree.ElementTree as ET
src, manifest_path, out = sys.argv[1:]
manifest=json.load(open(manifest_path,encoding='utf-8'))
ns='{http://www.iana.org/assignments}'
root=ET.parse(src).getroot()
def text(e): return (e.text or '').strip()
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"')
def child_text(e,name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
registries=[]; total=0
for reg in root.findall(ns+'registry'):
    rid=reg.get('id') or ''
    title=child_text(reg,'title')
    records=[]
    for rec in reg.findall(ns+'record'):
        fields=[]
        for c in list(rec):
            tag=c.tag.split('}',1)[-1]
            fields.append({'name':tag,'text':text(c),'attrs':dict(c.attrib)})
        xrefs=[]
        for x in rec.findall(ns+'xref'):
            xrefs.append({'type':x.get('type') or '', 'data':x.get('data') or '', 'text':text(x)})
        records.append({'name':child_text(rec,'name'),'description':child_text(rec,'description'),'note':child_text(rec,'note'),'xrefs':xrefs,'raw_fields':fields})
    total += len(records)
    registries.append({'id':rid,'title':title,'records':records})
def emit_attrs(attrs): return '[ ' + ' '.join('{ name = "%s"; value = "%s"; }' % (esc(k),esc(v)) for k,v in attrs.items()) + ' ]'
def emit_field(f): return '{ name = "%s"; text = "%s"; attrs = %s; }' % (esc(f['name']), esc(f['text']), emit_attrs(f['attrs']))
def emit_xref(x): return '{ type = "%s"; data = "%s"; text = "%s"; }' % (esc(x['type']), esc(x['data']), esc(x['text']))
def emit_record(r): return '{ name = "%s"; description = "%s"; note = "%s"; xrefs = [ %s ]; raw_fields = [ %s ]; }' % (esc(r['name']), esc(r['description']), esc(r['note']), ' '.join(emit_xref(x) for x in r['xrefs']), ' '.join(emit_field(f) for f in r['raw_fields']))
lines=[]
lines.append('# stdlib/lib/corpus/iana-http-parameters.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-http-parameters.sh')
lines.append('{')
lines.append('  schema = "registry.iana.http_parameters.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','source_url','source_path','retrieved_at_utc','source_sha256','license','version_policy']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  registries = [')
for reg in registries:
    lines.append('    { id = "%s"; title = "%s"; records = [ %s ]; }' % (esc(reg['id']), esc(reg['title']), ' '.join(emit_record(r) for r in reg['records'])))
lines.append('  ];')
lines.append('  registry_count = %d;' % len(registries))
lines.append('  record_count = %d;' % total)
lines.append('  generated_note = "IANA HTTP Parameters XML structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: registries={len(registries)} records={total}')
PY
