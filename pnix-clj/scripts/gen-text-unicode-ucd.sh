#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/text/unicode-ucd/UCD.zip"
OUT="$ROOT/stdlib/lib/corpus/unicode-ucd.generated.px"
CHUNK_DIR="$ROOT/stdlib/lib/corpus/unicode-ucd.generated"
CHUNK_SIZE="${UNICODE_UCD_CHUNK_SIZE:-2000}"
python3 - <<'PY' "$SRC" "$OUT" "$CHUNK_DIR" "$CHUNK_SIZE"
import json, pathlib, shutil, sys, zipfile
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); chunk_dir=pathlib.Path(sys.argv[3]); chunk_size=int(sys.argv[4])
FILES={
 'Blocks.txt':['range','block'],
 'Scripts.txt':['range','script'],
 'PropList.txt':['range','property'],
 'DerivedCoreProperties.txt':['range','derived_core_property'],
 'EastAsianWidth.txt':['range','east_asian_width'],
 'auxiliary/GraphemeBreakProperty.txt':['range','grapheme_break_property'],
 'PropertyAliases.txt':['property_alias','property_name','extra_aliases'],
 'PropertyValueAliases.txt':['property','value_alias','value_name','extra_aliases'],
}
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def split_line(line):
    raw=line.strip()
    if not raw or raw.startswith('#'): return None
    body=raw.split('#',1)[0].strip()
    if not body: return None
    return [p.strip() for p in body.split(';')]
def record_fields(cols, parts):
    fields=[]
    for i,c in enumerate(cols):
        if c=='extra_aliases': v=' | '.join(parts[i:]) if i < len(parts) else ''
        else: v=parts[i] if i < len(parts) else ''
        if v!='': fields.append((c,v))
    if len(parts)>len(cols) and (not cols or cols[-1] != 'extra_aliases'):
        fields.append(('extra_fields',' | '.join(parts[len(cols):])))
    return fields
def write_chunk(path, chunk_id, doc_path, cols, rows, start):
    lines=['{']
    lines.append('  schema = "text.unicode.ucd.v1";')
    lines.append('  source = { project = "Unicode Character Database"; license = "Unicode License v3"; retrieved_from = "https://www.unicode.org/Public/UCD/latest/ucd/UCD.zip"; };')
    lines.append(f'  chunk_id = {esc(chunk_id)};')
    lines.append(f'  document_path = {esc(doc_path)};')
    lines.append(f'  record_start = {start};')
    lines.append(f'  record_count = {len(rows)};')
    lines.append('  columns = [ ' + ' '.join(esc(c) for c in cols) + ' ];')
    lines.append('  records = [')
    for fields in rows:
        lines.append('    {')
        for k,v in fields:
            safe=k.replace('-', '_')
            lines.append(f'      {safe} = {esc(v)};')
        lines.append('    }')
    lines.append('  ];')
    lines.append('}')
    path.write_text('\n'.join(lines)+'\n', encoding='utf-8')
if chunk_dir.exists(): shutil.rmtree(chunk_dir)
chunk_dir.mkdir(parents=True)
chunks=[]; total=0; doc_meta=[]; idx=0
with zipfile.ZipFile(src) as z:
    for name,cols in FILES.items():
        rows=[]
        for line in z.read(name).decode('utf-8','replace').splitlines():
            parts=split_line(line)
            if parts is not None:
                rows.append(record_fields(cols,parts))
        doc_meta.append((name,cols,len(rows)))
        for start in range(0,len(rows),chunk_size):
            part=rows[start:start+chunk_size]
            chunk_id=f'ucd-{idx:04d}'
            rel=f'unicode-ucd.generated/{chunk_id}.px'
            write_chunk(chunk_dir/f'{chunk_id}.px', chunk_id, name, cols, part, start)
            chunks.append((chunk_id,rel,name,start,len(part)))
            idx+=1
        total+=len(rows)
lines=['{']
lines.append('  schema = "text.unicode.ucd.v1";')
lines.append('  source = { project = "Unicode Character Database"; license = "Unicode License v3"; retrieved_from = "https://www.unicode.org/Public/UCD/latest/ucd/UCD.zip"; };')
lines.append(f'  document_count = {len(doc_meta)};')
lines.append(f'  chunk_count = {len(chunks)};')
lines.append(f'  record_count = {total};')
lines.append('  documents = [')
for name,cols,count in doc_meta:
    lines.append(f'    {{ path = {esc(name)}; columns = [ ' + ' '.join(esc(c) for c in cols) + f' ]; record_count = {count}; }}')
lines.append('  ];')
lines.append('  chunks = [')
for cid,rel,name,start,count in chunks:
    lines.append(f'    {{ chunk_id = {esc(cid)}; path = {esc(rel)}; document_path = {esc(name)}; record_start = {start}; record_count = {count}; }}')
lines.append('  ];')
lines.append('}')
out.write_text('\n'.join(lines)+'\n', encoding='utf-8')
print(f'generated {out}: documents={len(doc_meta)} chunks={len(chunks)} records={total}')
for name,cols,count in doc_meta:
    print(f'  {name}: {count}')
PY
