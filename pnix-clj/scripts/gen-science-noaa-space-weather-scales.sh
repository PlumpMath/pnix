#!/usr/bin/env bash
# NOAA SWPC Space Weather Scales HTML -> pnix attrset source.
# Stores scale taxonomy/thresholds only. Excludes Effect prose and all forecast/alert feeds.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NOAA_SPACE_WEATHER_SCALES_SRC:-$ROOT/ingest/science/noaa-space-weather-scales/noaa-scales-explanation.html}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/noaa-space-weather-scales.generated.px}"
RECEIPT="$ROOT/ingest/science/noaa-space-weather-scales/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing NOAA SWPC scales HTML: $SRC" >&2
  echo "run scripts/update-science-noaa-space-weather-scales.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, re, hashlib
from pathlib import Path
from html.parser import HTMLParser
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
html=src.read_text(encoding='utf-8', errors='ignore')
class TableParser(HTMLParser):
    def __init__(self):
        super().__init__(); self.tables=[]; self.in_table=False; self.in_tr=False; self.in_cell=False; self.in_sup=False; self.cur_table=[]; self.cur_row=[]; self.cur_cell=[]
    def handle_starttag(self, tag, attrs):
        if tag=='table': self.in_table=True; self.cur_table=[]
        elif self.in_table and tag=='tr': self.in_tr=True; self.cur_row=[]
        elif self.in_tr and tag in ('td','th'): self.in_cell=True; self.cur_cell=[]
        elif self.in_cell and tag=='sup': self.in_sup=True; self.cur_cell.append('^')
    def handle_endtag(self, tag):
        if tag=='table' and self.in_table:
            self.tables.append(self.cur_table); self.in_table=False
        elif tag=='tr' and self.in_tr:
            if self.cur_row: self.cur_table.append(self.cur_row)
            self.in_tr=False
        elif tag in ('td','th') and self.in_cell:
            txt=' '.join(''.join(self.cur_cell).split())
            self.cur_row.append(txt); self.in_cell=False; self.in_sup=False
        elif tag=='sup': self.in_sup=False
    def handle_data(self, data):
        if self.in_cell: self.cur_cell.append(data)
p=TableParser(); p.feed(html)
scale_tables=[]
for t in p.tables:
    if not t or len(t[0]) < 5: continue
    head=t[0]
    if head[:3]==['Scale','Description','Effect']:
        scale_tables.append(t)
rows=[]
for idx,t in enumerate(scale_tables):
    head=t[0]
    measure_header=head[3]
    freq_header=head[4]
    for r in t[1:]:
        if len(r)<5: continue
        m=re.match(r'([GRS])\s*(\d+)$', r[0])
        if not m: continue
        family,level=m.group(1),int(m.group(2))
        rows.append({
          'scale':family,
          'level':level,
          'code':f'{family}{level}',
          'severity':r[1],
          'physical_measure_header':measure_header,
          'physical_measure':r[3],
          'average_frequency_header':freq_header,
          'average_frequency':r[4],
          'source_table_index':idx,
        })
counts={}
for r in rows: counts[r['scale']]=counts.get(r['scale'],0)+1
obj={
 'schema':'science.noaa.space_weather_scales.v1',
 'source':{
   'name':'NOAA SWPC Space Weather Scales',
   'license':'US Government public domain',
   'source_urls':['https://www.spaceweather.gov/noaa-scales-explanation','https://www.swpc.noaa.gov/noaa-scales-explanation'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-noaa-space-weather-scales.sh',
   'scope':'G/R/S scale taxonomy and physical threshold metadata only; effects prose/forecast feeds/alerts/operational instructions excluded'
 },
 'source_files':{'html_sha256':hashlib.sha256(html.encode()).hexdigest()},
 'summary':{'scale_count':len(rows),'family_count':len(counts),'scales_by_family':counts,'effect_prose_excluded':True},
 'scales':sorted(rows, key=lambda x:(x['scale'],-x['level'])),
}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/noaa-space-weather-scales.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-noaa-space-weather-scales.sh && scripts/gen-science-noaa-space-weather-scales.sh\n'
content+='# 범위: NOAA G/R/S scale taxonomy + physical thresholds only. Effect prose/forecast/alert/ops instruction 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: scales={len(rows)} families={len(counts)} bytes={len(content.encode())}")
PY
