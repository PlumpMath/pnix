#!/usr/bin/env bash
# IANA Character Sets XML -> pnix attrset source. 기관 XML registry schema를 보존한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/registry/iana-character-sets/character-sets.xml"
MANIFEST="$ROOT/ingest/registry/iana-character-sets/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-character-sets.generated.px"
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
manifest=json.load(open(manifest_path, encoding='utf-8'))
ns='{http://www.iana.org/assignments}'
root=ET.parse(src).getroot()
def text(e): return (e.text or '').strip()
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"')
def child_text(e, name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
registries=[]
record_count=0
for reg in root.findall(ns+'registry'):
    rid=reg.get('id') or ''
    title=child_text(reg,'title')
    records=[]
    for rec in reg.findall(ns+'record'):
        aliases=[text(a) for a in rec.findall(ns+'alias') if text(a)]
        xrefs=[]
        for x in rec.findall(ns+'xref'):
            xrefs.append({'type':x.get('type') or '', 'data':x.get('data') or '', 'text':text(x)})
        records.append({
            'name': child_text(rec,'name'),
            'value': child_text(rec,'value'),
            'description': child_text(rec,'description'),
            'aliases': aliases,
            'xrefs': xrefs,
        })
    record_count += len(records)
    registries.append({'id':rid,'title':title,'records':records})
def emit_xref(x):
    return '{ type = "%s"; data = "%s"; text = "%s"; }' % (esc(x['type']), esc(x['data']), esc(x['text']))
def emit_record(r):
    aliases=' '.join('"%s"' % esc(a) for a in r['aliases'])
    return '{ name = "%s"; value = "%s"; description = "%s"; aliases = [ %s ]; xrefs = [ %s ]; }' % (esc(r['name']), esc(r['value']), esc(r['description']), aliases, ' '.join(emit_xref(x) for x in r['xrefs']))
lines=[]
lines.append('# stdlib/lib/corpus/iana-character-sets.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-character-sets.sh')
lines.append('{')
lines.append('  schema = "registry.iana.character_sets.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','source_url','source_path','retrieved_at_utc','source_sha256','license','version_policy']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  registries = [')
for reg in registries:
    lines.append('    {')
    lines.append('      id = "%s";' % esc(reg['id']))
    lines.append('      title = "%s";' % esc(reg['title']))
    lines.append('      records = [')
    for r in reg['records']:
        lines.append('        ' + emit_record(r))
    lines.append('      ];')
    lines.append('    }')
lines.append('  ];')
lines.append('  registry_count = %d;' % len(registries))
lines.append('  record_count = %d;' % record_count)
lines.append('  generated_note = "IANA XML registry structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: registries={len(registries)} records={record_count}')
PY
