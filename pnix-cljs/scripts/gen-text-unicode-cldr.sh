#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/text/unicode-cldr/core.zip"
OUT="$ROOT/stdlib/lib/corpus/unicode-cldr.generated.px"
CHUNK_DIR="$ROOT/stdlib/lib/corpus/unicode-cldr.generated"
CHUNK_SIZE="${UNICODE_CLDR_CHUNK_SIZE:-1000}"
python3 - <<'PY' "$SRC" "$OUT" "$CHUNK_DIR" "$CHUNK_SIZE"
import json, pathlib, shutil, sys, zipfile, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); chunk_dir=pathlib.Path(sys.argv[3]); chunk_size=int(sys.argv[4])
FILES=['common/supplemental/likelySubtags.xml','common/supplemental/numberingSystems.xml','common/supplemental/supplementalData.xml','common/bcp47/calendar.xml','common/bcp47/number.xml','common/validity/language.xml','common/validity/region.xml']
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def local(tag): return tag.rsplit('}',1)[-1]
def clean_text(e): return ' '.join(''.join(e.itertext()).split())
def rows_for(root):
    rows=[]
    def rec(e,parent):
        tag=local(e.tag); cur=parent+'/'+tag if parent else tag; attrs=sorted((local(k),v) for k,v in e.attrib.items()); text=clean_text(e); child_count=len(list(e))
        if attrs or (text and child_count==0): rows.append({'tag':tag,'path':cur,'attrs':attrs,'text':text if child_count==0 else ''})
        for ch in list(e): rec(ch,cur)
    rec(root,''); return rows
def write_chunk(path, cid, doc_path, rows, start):
    lines=['{']
    lines.append('  schema = "text.unicode.cldr.v1";')
    lines.append('  source = { project = "Unicode CLDR Core"; license = "Unicode License v3"; retrieved_from = "https://www.unicode.org/Public/cldr/latest/core.zip"; };')
    lines.append(f'  chunk_id = {esc(cid)};')
    lines.append(f'  document_path = {esc(doc_path)};')
    lines.append(f'  record_start = {start};')
    lines.append(f'  record_count = {len(rows)};')
    lines.append('  records = [')
    for r in rows:
        lines.append('    {')
        lines.append(f'      tag = {esc(r["tag"])};')
        lines.append(f'      path = {esc(r["path"])};')
        if r['attrs']:
            lines.append('      attrs = {')
            for k,v in r['attrs']:
                lines.append(f'        {esc(k)} = {esc(v)};')
            lines.append('      };')
        if r['text']:
            lines.append(f'      text = {esc(r["text"])};')
        lines.append('    }')
    lines.append('  ];')
    lines.append('}')
    path.write_text('\n'.join(lines)+'\n', encoding='utf-8')
if chunk_dir.exists(): shutil.rmtree(chunk_dir)
chunk_dir.mkdir(parents=True)
docs=[]; chunks=[]; total=0; idx=0
with zipfile.ZipFile(src) as z:
    for name in FILES:
        rows=rows_for(ET.fromstring(z.read(name)))
        docs.append((name,len(rows))); total+=len(rows)
        for start in range(0,len(rows),chunk_size):
            part=rows[start:start+chunk_size]
            cid=f'cldr-{idx:04d}'; rel=f'unicode-cldr.generated/{cid}.px'
            write_chunk(chunk_dir/f'{cid}.px', cid, name, part, start)
            chunks.append((cid,rel,name,start,len(part))); idx+=1
lines=['{']
lines.append('  schema = "text.unicode.cldr.v1";')
lines.append('  source = { project = "Unicode CLDR Core"; license = "Unicode License v3"; retrieved_from = "https://www.unicode.org/Public/cldr/latest/core.zip"; };')
lines.append(f'  document_count = {len(docs)};')
lines.append(f'  chunk_count = {len(chunks)};')
lines.append(f'  record_count = {total};')
lines.append('  documents = [')
for name,count in docs: lines.append(f'    {{ path = {esc(name)}; record_count = {count}; }}')
lines.append('  ];')
lines.append('  chunks = [')
for cid,rel,name,start,count in chunks: lines.append(f'    {{ chunk_id = {esc(cid)}; path = {esc(rel)}; document_path = {esc(name)}; record_start = {start}; record_count = {count}; }}')
lines.append('  ];')
lines.append('}')
out.write_text('\n'.join(lines)+'\n', encoding='utf-8')
print(f'generated {out}: documents={len(docs)} chunks={len(chunks)} records={total}')
for name,count in docs: print(f'  {name}: {count}')
PY
