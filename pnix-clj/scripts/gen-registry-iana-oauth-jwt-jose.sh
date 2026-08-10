#!/usr/bin/env bash
# IANA OAuth/JWT/JOSE XML bundle -> pnix attrset source. 기관 XML field schema를 보존한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-oauth-jwt-jose"
MANIFEST="$SRC_DIR/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-oauth-jwt-jose.generated.px"
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
def esc(s):
    return (s or '').replace('\\','\\\\').replace('"','\\"').replace('\r',' ').replace('\n',' ').replace('\t',' ')
def child_text(e,name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
bundles=[]; total_regs=0; total_records=0
for src in manifest.get('sources',[]):
    path=os.path.join(src_dir, src['source_path'])
    root=ET.parse(path).getroot()
    regs=[]
    for reg in root.findall(ns+'registry'):
        rid=reg.get('id') or ''
        title=child_text(reg,'title')
        records=[]
        for rec in reg.findall(ns+'record'):
            xrefs=[]
            for x in rec.findall(ns+'xref'):
                xrefs.append({'type':x.get('type') or '', 'data':x.get('data') or '', 'section':x.get('section') or ''})
            records.append({
                'name': child_text(rec,'name'),
                'value': child_text(rec,'value'),
                'controller': child_text(rec,'controller'),
                'usage': child_text(rec,'usage'),
                'requirements': child_text(rec,'requirements'),
                'xrefs': xrefs,
            })
        total_records += len(records)
        regs.append({'id':rid,'title':title,'records':records})
    total_regs += len(regs)
    bundles.append({'source_id':src['source_id'],'source_url':src['source_url'],'source_sha256':src['source_sha256'],'registries':regs})
def emit_attrs(attrs): return '[ ' + ' '.join('{ name = "%s"; value = "%s"; }' % (esc(k),esc(v)) for k,v in attrs.items()) + ' ]'
def emit_xref(x): return '{ type = "%s"; data = "%s"; section = "%s"; }' % (esc(x['type']), esc(x['data']), esc(x['section']))
def emit_record(r): return '{ name = "%s"; value = "%s"; controller = "%s"; usage = "%s"; requirements = "%s"; xrefs = [ %s ]; }' % (esc(r['name']), esc(r['value']), esc(r['controller']), esc(r['usage']), esc(r['requirements']), ' '.join(emit_xref(x) for x in r['xrefs']))
lines=[]
lines.append('# stdlib/lib/corpus/iana-oauth-jwt-jose.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-oauth-jwt-jose.sh')
lines.append('{')
lines.append('  schema = "registry.iana.oauth_jwt_jose.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','retrieved_at_utc','license','version_policy']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('    sources = [')
for s in manifest.get('sources',[]):
    lines.append('      { source_id = "%s"; source_url = "%s"; source_path = "%s"; source_sha256 = "%s"; }' % (esc(s['source_id']), esc(s['source_url']), esc(s['source_path']), esc(s['source_sha256'])))
lines.append('    ];')
lines.append('  };')
lines.append('  bundles = [')
for b in bundles:
    lines.append('    {')
    lines.append('      source_id = "%s";' % esc(b['source_id']))
    lines.append('      source_url = "%s";' % esc(b['source_url']))
    lines.append('      source_sha256 = "%s";' % esc(b['source_sha256']))
    lines.append('      registries = [')
    for reg in b['registries']:
        lines.append('        { id = "%s"; title = "%s"; records = [' % (esc(reg['id']), esc(reg['title'])))
        for r in reg['records']:
            lines.append('          %s' % emit_record(r))
        lines.append('        ]; }')
    lines.append('      ];')
    lines.append('    }')
lines.append('  ];')
lines.append('  bundle_count = %d;' % len(bundles))
lines.append('  registry_count = %d;' % total_regs)
lines.append('  record_count = %d;' % total_records)
lines.append('  generated_note = "IANA OAuth/JWT/JOSE XML structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: bundles={len(bundles)} registries={total_regs} records={total_records}')
PY
