#!/usr/bin/env bash
# USGS Geomagnetism Observatories HTML -> pnix attrset source.
# Stores observatory code/name/page URL only. Excludes observations, realtime feeds, alerts, operational decisions.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USGS_GEOMAG_OBSERVATORIES_SRC:-$ROOT/ingest/science/usgs-geomagnetism-observatories/observatories.html}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-geomagnetism-observatories.generated.px}"
RECEIPT="$ROOT/ingest/science/usgs-geomagnetism-observatories/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing USGS Geomagnetism Observatories HTML: $SRC" >&2
  echo "run scripts/update-science-usgs-geomagnetism-observatories.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, re, hashlib, html as H
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
text=src.read_text(encoding='utf-8', errors='ignore')
items=[]
pat=re.compile(r'<a\s+href="([^"]+)"[^>]*>\s*([A-Z]{3})\s+-\s*([^<]+)</a>')
for href,code,name in pat.findall(text):
    if '/geomagnetism/science/' not in href: continue
    url=href if href.startswith('http') else 'https://www.usgs.gov'+href
    items.append({'code':code,'name':H.unescape(name).strip(),'official_page_url':url})
seen={}; dedup=[]
for x in items:
    if x['code'] not in seen:
        seen[x['code']]=1; dedup.append(x)
dedup=sorted(dedup, key=lambda x:x['code'])
obj={
 'schema':'science.usgs.geomagnetism_observatories.v1',
 'source':{
   'name':'USGS Geomagnetism Observatories',
   'license':'US Government public domain',
   'source_urls':['https://www.usgs.gov/programs/geomagnetism/science/observatories'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-usgs-geomagnetism-observatories.sh',
   'scope':'observatory code/name/page URL metadata only; geomagnetic time-series/realtime feeds/alerts/operational decisions excluded'
 },
 'source_files':{'html_sha256':hashlib.sha256(text.encode()).hexdigest()},
 'summary':{'observatory_count':len(dedup),'kmz_coordinates_ingested':False,'kmz_download_status':receipt.get('kmz_download_status')},
 'observatories':dedup,
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
content='# stdlib/lib/corpus/usgs-geomagnetism-observatories.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-geomagnetism-observatories.sh && scripts/gen-science-usgs-geomagnetism-observatories.sh\n'
content+='# 범위: USGS geomagnetic observatory code/name/page URL metadata only. observations/realtime/alerts/ops decisions 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: observatories={len(dedup)} bytes={len(content.encode())}")
PY
