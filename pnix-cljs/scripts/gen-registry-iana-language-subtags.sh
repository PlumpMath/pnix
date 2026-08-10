#!/usr/bin/env bash
# IANA Language Subtag Registry text -> pnix attrset source. 원본 field schema를 보존한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/registry/iana-language-subtags/language-subtag-registry.txt"
MANIFEST="$ROOT/ingest/registry/iana-language-subtags/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/iana-language-subtags.generated.px"
CHUNK_SIZE=1000
CHUNK_INDEX=0
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --manifest) MANIFEST="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --chunk-size) CHUNK_SIZE="$2"; shift 2;;
    --chunk-index) CHUNK_INDEX="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$MANIFEST" "$OUT" "$CHUNK_SIZE" "$CHUNK_INDEX" <<'PY'
import json, sys
src, manifest_path, out, chunk_size, chunk_index = sys.argv[1:]
chunk_size, chunk_index = int(chunk_size), int(chunk_index)
manifest=json.load(open(manifest_path, encoding='utf-8'))
text=open(src,encoding='utf-8').read()
parts=[p.strip('\n') for p in text.split('%%') if p.strip()]
header={}
records=[]
def add_field(obj,k,v):
    obj.setdefault(k,[]).append(v)
for i,part in enumerate(parts):
    current_key=None
    obj={}
    for line in part.splitlines():
        if not line: continue
        if line.startswith(' ') and current_key:
            obj[current_key][-1] += ' ' + line.strip()
        elif ':' in line:
            k,v=line.split(':',1)
            current_key=k.strip()
            add_field(obj,current_key,v.strip())
    if i==0 and 'File-Date' in obj:
        header=obj
    else:
        records.append(obj)
source_record_count=len(records)
start=chunk_index*chunk_size
end=start+chunk_size
chunk=records[start:end]
def esc(s): return (s or '').replace('\\','\\\\').replace('"','\\"')
def emit_str_list(xs): return '[ ' + ' '.join('"%s"' % esc(x) for x in xs) + ' ]'
def one(obj,k):
    vals=obj.get(k,[])
    return vals[0] if vals else ''
def emit_record(r):
    # Preserve common BCP47 fields explicitly, and keep every field in raw_fields as field -> [values].
    raw=' '.join('{ name = "%s"; values = %s; }' % (esc(k), emit_str_list(v)) for k,v in r.items())
    return ('{ type = "%s"; subtag = "%s"; tag = "%s"; description = %s; added = "%s"; deprecated = "%s"; preferred_value = "%s"; suppress_script = "%s"; macrolanguage = "%s"; raw_fields = [ %s ]; }' % (
        esc(one(r,'Type')), esc(one(r,'Subtag')), esc(one(r,'Tag')), emit_str_list(r.get('Description',[])), esc(one(r,'Added')), esc(one(r,'Deprecated')), esc(one(r,'Preferred-Value')), esc(one(r,'Suppress-Script')), esc(one(r,'Macrolanguage')), raw))
lines=[]
lines.append('# stdlib/lib/corpus/iana-language-subtags.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-registry-iana-language-subtags.sh')
lines.append('{')
lines.append('  schema = "registry.iana.language_subtags.v1";')
lines.append('  source = {')
for k in ['source_id','project','snapshot_kind','source_url','source_path','file_date','retrieved_at_utc','source_sha256','license','version_policy']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('  };')
lines.append('  shard = { chunk_size = %d; chunk_index = %d; source_record_count = %d; };' % (chunk_size, chunk_index, source_record_count))
lines.append('  records = [')
for r in chunk:
    lines.append('    ' + emit_record(r))
lines.append('  ];')
lines.append('  record_count = %d;' % len(chunk))
lines.append('  generated_note = "IANA Language Subtag Registry structural transcription only; chunked redb source row; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: chunk={chunk_index} records={len(chunk)} source_records={source_record_count}')
PY
