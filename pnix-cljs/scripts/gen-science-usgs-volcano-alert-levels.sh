#!/usr/bin/env bash
# USGS Volcano Alert Level System HTML -> static taxonomy pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USGS_VOLCANO_ALERT_LEVELS_SRC:-$ROOT/ingest/science/usgs-volcano-alert-levels/alert-level-system.html}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-volcano-alert-levels.generated.px}"
RECEIPT="$ROOT/ingest/science/usgs-volcano-alert-levels/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing USGS Volcano Alert Level System HTML: $SRC" >&2
  echo "run scripts/update-science-usgs-volcano-alert-levels.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, hashlib
from pathlib import Path
from html.parser import HTMLParser
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
html=src.read_text(encoding='utf-8', errors='ignore')
class TableParser(HTMLParser):
    def __init__(self):
        super().__init__(); self.tables=[]; self.in_table=False; self.in_tr=False; self.in_cell=False; self.cur_table=[]; self.cur_row=[]; self.cur_cell=[]
    def handle_starttag(self, tag, attrs):
        if tag=='table': self.in_table=True; self.cur_table=[]
        elif self.in_table and tag=='tr': self.in_tr=True; self.cur_row=[]
        elif self.in_tr and tag in ('td','th'): self.in_cell=True; self.cur_cell=[]
    def handle_endtag(self, tag):
        if tag=='table' and self.in_table:
            self.tables.append(self.cur_table); self.in_table=False
        elif tag=='tr' and self.in_tr:
            if self.cur_row: self.cur_table.append(self.cur_row)
            self.in_tr=False
        elif tag in ('td','th') and self.in_cell:
            self.cur_row.append(' '.join(''.join(self.cur_cell).split())); self.in_cell=False
    def handle_data(self,data):
        if self.in_cell: self.cur_cell.append(data)
p=TableParser(); p.feed(html)
alert_terms=['NORMAL','ADVISORY','WATCH','WARNING']
color_terms=['GREEN','YELLOW','ORANGE','RED']
pairs={'NORMAL':'GREEN','ADVISORY':'YELLOW','WATCH':'ORANGE','WARNING':'RED'}
alert_desc={}; color_desc={}
for t in p.tables:
    for row in t[1:]:
        if len(row) >= 2 and row[0] in alert_terms: alert_desc[row[0]]=row[1]
        if len(row) >= 2 and row[0] in color_terms: color_desc[row[0]]=row[1]
def dh(s): return hashlib.sha256((s or '').encode('utf-8')).hexdigest() if s else None
alert_levels=[]
for i,term in enumerate(alert_terms):
    alert_levels.append({'term':term,'rank':i,'scope':'ground_hazards','notification_type':'VAN','paired_aviation_color_code':pairs[term],'description_sha256':dh(alert_desc.get(term)),'description_present':term in alert_desc})
aviation_colors=[]
for i,term in enumerate(color_terms):
    inv={v:k for k,v in pairs.items()}
    aviation_colors.append({'code':term,'rank':i,'scope':'aviation_ash_hazards','notification_type':'VONA','paired_volcano_alert_level':inv[term],'description_sha256':dh(color_desc.get(term)),'description_present':term in color_desc})
obj={
 'schema':'science.usgs.volcano_alert_levels.v1',
 'source':{
   'name':'USGS Volcano Alert Level System',
   'license':'US Government public domain',
   'source_urls':['https://www.usgs.gov/programs/VHP/alert-level-system','https://www.usgs.gov/programs/VHP/alert-level-icons'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-usgs-volcano-alert-levels.sh',
   'scope':'static volcano alert-level and aviation-color-code taxonomy only; live notices/events/prose guidance excluded'
 },
 'source_files':{'html_sha256':hashlib.sha256(html.encode()).hexdigest()},
 'summary':{'alert_level_count':len(alert_levels),'aviation_color_count':len(aviation_colors),'description_text_ingested':False,'live_notices_ingested':False},
 'volcano_alert_levels':alert_levels,
 'aviation_color_codes':aviation_colors,
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
content='# stdlib/lib/corpus/usgs-volcano-alert-levels.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-volcano-alert-levels.sh && scripts/gen-science-usgs-volcano-alert-levels.sh\n'
content+='# 범위: USGS Volcano Alert Level static taxonomy only. live notices/events/prose guidance 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: alert_levels={len(alert_levels)} aviation_colors={len(aviation_colors)} bytes={len(content.encode())}")
PY
