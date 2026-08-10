#!/usr/bin/env bash
# IANA Media Types XML -> pnix attrset source. 기관 XML registry schema를 보존한다.
# 큰 registry는 chunk로 나눠 redb append-only row 여러 개로 적재한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/registry/iana-media-types/media-types.xml"
MANIFEST="$ROOT/ingest/registry/iana-media-types/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-media-types.generated.px"
REGISTRY="all"
CHUNK_SIZE=500
CHUNK_INDEX=0
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --manifest) MANIFEST="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --registry) REGISTRY="$2"; shift 2;;
    --chunk-size) CHUNK_SIZE="$2"; shift 2;;
    --chunk-index) CHUNK_INDEX="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$MANIFEST" "$OUT" "$REGISTRY" "$CHUNK_SIZE" "$CHUNK_INDEX" <<'PY'
import json, sys, xml.etree.ElementTree as ET
src, manifest_path, out, want_registry, chunk_size, chunk_index = sys.argv[1:]
chunk_size, chunk_index = int(chunk_size), int(chunk_index)
manifest=json.load(open(manifest_path, encoding='utf-8'))
ns='{http://www.iana.org/assignments}'
root=ET.parse(src).getroot()
def text(e): return (e.text or '').strip()
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"')
def child_text(e, name):
    c=e.find(ns+name)
    return text(c) if c is not None else ''
registries=[]
source_registry_count=0
source_record_count=0
for reg in root.findall(ns+'registry'):
    rid=reg.get('id') or ''
    if want_registry != 'all' and rid != want_registry:
        continue
    title=child_text(reg,'title')
    all_records=[]
    for rec in reg.findall(ns+'record'):
        name=child_text(rec,'name')
        file_el=rec.find(ns+'file')
        files=[]
        if file_el is not None:
            files.append({'type':file_el.get('type') or '', 'path':text(file_el)})
        xrefs=[]
        for x in rec.findall(ns+'xref'):
            xrefs.append({'type':x.get('type') or '', 'data':x.get('data') or '', 'text':text(x)})
        all_records.append({'name':name,'files':files,'xrefs':xrefs})
    source_registry_count += 1
    source_record_count += len(all_records)
    start = chunk_index * chunk_size
    end = start + chunk_size
    records = all_records[start:end]
    if records:
        registries.append({'id':rid,'title':title,'records':records,'source_record_count':len(all_records)})
record_count=sum(len(r['records']) for r in registries)
def emit_xref(x):
    return '{ type = "%s"; data = "%s"; text = "%s"; }' % (esc(x['type']), esc(x['data']), esc(x['text']))
def emit_file(f):
    return '{ type = "%s"; path = "%s"; }' % (esc(f['type']), esc(f['path']))
def emit_record(r):
    return '{ name = "%s"; files = [ %s ]; xrefs = [ %s ]; }' % (esc(r['name']), ' '.join(emit_file(f) for f in r['files']), ' '.join(emit_xref(x) for x in r['xrefs']))
lines=[]
lines.append('# stdlib/lib/corpus/iana-media-types.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-media-types.sh')
lines.append('{')
lines.append('  schema = "registry.iana.media_types.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','source_url','source_path','retrieved_at_utc','source_sha256','license','version_policy']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  shard = { registry = "%s"; chunk_size = %d; chunk_index = %d; source_registry_count = %d; source_record_count = %d; };' % (esc(want_registry), chunk_size, chunk_index, source_registry_count, source_record_count))
lines.append('  registries = [')
for reg in registries:
    lines.append('    {')
    lines.append('      id = "%s";' % esc(reg['id']))
    lines.append('      title = "%s";' % esc(reg['title']))
    lines.append('      source_record_count = %d;' % reg['source_record_count'])
    lines.append('      records = [')
    for r in reg['records']:
        lines.append('        ' + emit_record(r))
    lines.append('      ];')
    lines.append('    }')
lines.append('  ];')
lines.append('  registry_count = %d;' % len(registries))
lines.append('  record_count = %d;' % record_count)
lines.append('  generated_note = "IANA XML registry structural transcription only; chunked redb source row; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: registry={want_registry} chunk={chunk_index} registries={len(registries)} records={record_count} source_records={source_record_count}')
PY
