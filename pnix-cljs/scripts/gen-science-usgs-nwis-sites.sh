#!/usr/bin/env bash
# USGS NWIS Site Service RDB -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USGS_NWIS_SITES_SRC:-$ROOT/ingest/science/usgs-nwis-sites/nwis-sites.rdb}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-nwis-sites.generated.px}"
RECEIPT="$ROOT/ingest/science/usgs-nwis-sites/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing USGS NWIS sites RDB: $SRC" >&2
  echo "run scripts/update-science-usgs-nwis-sites.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, hashlib
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
text=src.read_text(encoding='utf-8', errors='replace')
lines=[l for l in text.splitlines() if l and not l.startswith('#')]
header=lines[0].split('\t') if lines else []
rows=[]
def val(d,k):
    v=d.get(k)
    return None if v in (None,'') else v
def num(v):
    if v in (None,''): return None
    try: return float(v)
    except Exception: return v
for l in lines[2:]:
    parts=l.split('\t')
    d=dict(zip(header,parts))
    rows.append({
      'agency_cd':val(d,'agency_cd'),
      'site_no':val(d,'site_no'),
      'station_nm':val(d,'station_nm'),
      'site_tp_cd':val(d,'site_tp_cd'),
      'dec_lat_va':num(val(d,'dec_lat_va')),
      'dec_long_va':num(val(d,'dec_long_va')),
      'coord_acy_cd':val(d,'coord_acy_cd'),
      'dec_coord_datum_cd':val(d,'dec_coord_datum_cd'),
      'alt_va':num(val(d,'alt_va')),
      'alt_acy_va':num(val(d,'alt_acy_va')),
      'alt_datum_cd':val(d,'alt_datum_cd'),
      'huc_cd':val(d,'huc_cd'),
    })
rows=sorted(rows,key=lambda x:x['site_no'] or '')
site_types={}; agencies={}
for r in rows:
    site_types[r.get('site_tp_cd') or 'unknown']=site_types.get(r.get('site_tp_cd') or 'unknown',0)+1
    agencies[r.get('agency_cd') or 'unknown']=agencies.get(r.get('agency_cd') or 'unknown',0)+1
obj={
 'schema':'science.usgs.nwis_sites.v1',
 'source':{
   'name':'USGS NWIS Site Service monitoring-location metadata',
   'license':'US Government public domain',
   'source_urls':['https://waterservices.usgs.gov/nwis/site/','https://waterservices.usgs.gov/'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-usgs-nwis-sites.sh',
   'scope':'bounded NWIS site metadata only; observations/time-series/water-quality/private wells/water-rights/safety judgments excluded'
 },
 'source_files':{'rdb_sha256':hashlib.sha256(text.encode()).hexdigest()},
 'summary':{'site_count':len(rows),'site_types':site_types,'agencies':agencies,'observations_ingested':False,'water_quality_ingested':False,'safety_judgments_ingested':False},
 'fields':['agency_cd','site_no','station_nm','site_tp_cd','dec_lat_va','dec_long_va','coord_acy_cd','dec_coord_datum_cd','alt_va','alt_acy_va','alt_datum_cd','huc_cd'],
 'sites':rows,
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
content='# stdlib/lib/corpus/usgs-nwis-sites.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-nwis-sites.sh && scripts/gen-science-usgs-nwis-sites.sh\n'
content+='# 범위: bounded USGS NWIS site metadata only. observations/time-series/water-quality/safety judgments 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: sites={len(rows)} bytes={len(content.encode())}")
PY
