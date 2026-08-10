#!/usr/bin/env bash
# NASA Exoplanet Archive selected pscomppars CSV -> one pnix attrset chunk.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NASA_EXOPLANET_ARCHIVE_SRC:-$ROOT/ingest/science/nasa-exoplanet-archive/pscomppars-selected.csv}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/nasa-exoplanet-archive.generated.px}"
RECEIPT="$ROOT/ingest/science/nasa-exoplanet-archive/source-receipt.json"
CHUNK_SIZE=1000
CHUNK_INDEX=0
while [ $# -gt 0 ]; do
  case "$1" in
    --chunk-size) CHUNK_SIZE="$2"; shift 2;;
    --chunk-index) CHUNK_INDEX="$2"; shift 2;;
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
if [[ ! -f "$SRC" ]]; then
  echo "missing NASA Exoplanet Archive CSV: $SRC" >&2
  echo "run scripts/update-science-nasa-exoplanet-archive.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" "$CHUNK_SIZE" "$CHUNK_INDEX" <<'PY'
import csv, json, sys, hashlib, math
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3]); chunk_size=int(sys.argv[4]); chunk_index=int(sys.argv[5])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
text=src.read_text(encoding='utf-8', errors='replace')
fields=['pl_name','hostname','discoverymethod','disc_year','disc_facility','pl_orbper','pl_orbsmax','pl_orbeccen','pl_rade','pl_bmasse','pl_eqt','st_teff','st_rad','st_mass','sy_dist','sy_snum','sy_pnum']
number_fields={'disc_year','pl_orbper','pl_orbsmax','pl_orbeccen','pl_rade','pl_bmasse','pl_eqt','st_teff','st_rad','st_mass','sy_dist','sy_snum','sy_pnum'}
def coerce(k,v):
    if v is None or v=='': return None
    if k in number_fields:
        try:
            if k in {'disc_year','sy_snum','sy_pnum'}: return int(float(v))
            return float(v)
        except Exception:
            return v
    return v
all_rows=[]; methods={}; years={}
for row in csv.DictReader(text.splitlines()):
    item={k:coerce(k,row.get(k,'')) for k in fields}
    all_rows.append(item)
    m=item.get('discoverymethod') or 'unknown'; methods[m]=methods.get(m,0)+1
    y=item.get('disc_year')
    if y is not None: years[str(y)]=years.get(str(y),0)+1
total=len(all_rows)
chunk_count=(total + chunk_size - 1)//chunk_size if chunk_size > 0 else 1
if chunk_index < 0 or chunk_index >= chunk_count:
    raise SystemExit(f'chunk-index out of range: {chunk_index} / {chunk_count}')
start=chunk_index*chunk_size; end=min(total,start+chunk_size)
rows=all_rows[start:end]
obj={
 'schema':'science.nasa.exoplanet_archive.pscomppars.v2',
 'source':{
   'name':'NASA Exoplanet Archive confirmed planets composite parameters',
   'license':'NASA public data / acknowledgment required',
   'source_urls':['https://exoplanetarchive.ipac.caltech.edu/','https://exoplanetarchive.ipac.caltech.edu/TAP/sync','https://exoplanetarchive.ipac.caltech.edu/docs/acknowledge.html'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-nasa-exoplanet-archive.sh',
   'scope':'selected pscomppars table fields only; literature text/comments/light curves/spectra/images/time-series/prediction/graph wiring excluded'
 },
 'source_files':{'csv_sha256':hashlib.sha256(text.encode()).hexdigest()},
 'summary':{'row_count':len(rows),'total_row_count':total,'field_count':len(fields),'discovery_methods':methods,'disc_year_counts':years},
 'chunk':{'index':chunk_index,'count':chunk_count,'size':chunk_size,'start':start,'end':end},
 'fields':fields,
 'rows':rows,
}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v, allow_nan=False)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/nasa-exoplanet-archive.generated.px — GENERATED chunk, do not commit.\n'
content+='# 생성: scripts/update-science-nasa-exoplanet-archive.sh && scripts/gen-science-nasa-exoplanet-archive.sh\n'
content+='# 범위: selected NASA Exoplanet Archive pscomppars table fields only. prose/light curves/spectra/images/time-series/prediction 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: chunk={chunk_index+1}/{chunk_count} rows={len(rows)} total={total} fields={len(fields)} bytes={len(content.encode())}")
PY
