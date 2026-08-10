#!/usr/bin/env bash
# USDA NRCS Soil Data Access / SSURGO sacatalog JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USDA_SSURGO_SACATALOG_SRC:-$ROOT/ingest/science/usda-ssurgo-sacatalog/sacatalog.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usda-ssurgo-sacatalog.generated.px}"
RECEIPT="$ROOT/ingest/science/usda-ssurgo-sacatalog/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing SSURGO sacatalog JSON: $SRC" >&2
  echo "run scripts/update-science-usda-ssurgo-sacatalog.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import hashlib, json, pathlib, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:])
raw=src.read_bytes()
obj=json.loads(raw.decode('utf-8'))
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
rows=[]
for r in obj.get('Table') or []:
    rows.append({
      'areasymbol': r[0] if len(r)>0 and r[0] != '' else None,
      'areaname': r[1] if len(r)>1 and r[1] != '' else None,
      'saversion': int(r[2]) if len(r)>2 and str(r[2]).isdigit() else (r[2] if len(r)>2 else None),
      'saverest': r[3] if len(r)>3 and r[3] != '' else None,
    })
rows=sorted(rows,key=lambda x:x.get('areasymbol') or '')
by_prefix={}
for row in rows:
    sym=row.get('areasymbol') or ''
    prefix=''.join(c for c in sym[:2] if c.isalpha()) or 'unknown'
    by_prefix[prefix]=by_prefix.get(prefix,0)+1
out_obj={
  'schema':'science.usda.ssurgo_sacatalog.v1',
  'source':{
    'name':'USDA NRCS Soil Data Access / SSURGO sacatalog survey-area catalog',
    'license':'US Government public domain',
    'source_urls':['https://sdmdataaccess.nrcs.usda.gov/','https://sdmdataaccess.nrcs.usda.gov/webservicehelp.aspx','https://www.nrcs.usda.gov/resources/data-and-reports/soil-survey-geographic-database-ssurgo','https://sdmdataaccess.sc.egov.usda.gov/Tabular/post.rest'],
    'receipt':receipt,
    'generated_at':receipt.get('retrieved_at'),
    'generator':'scripts/gen-science-usda-ssurgo-sacatalog.sh',
    'scope':'SSURGO sacatalog survey-area catalog metadata only; FGDC XML/prose, soil property tables, geometry, interpretations/ratings, and suitability judgments excluded'
  },
  'source_files':{'sacatalog_json_sha256':hashlib.sha256(raw).hexdigest()},
  'summary':{
    'survey_area_count':len(rows),
    'area_prefix_counts':by_prefix,
    'fgdc_metadata_ingested':False,
    'soil_geometry_ingested':False,
    'mapunit_component_horizon_ingested':False,
    'interpretations_ratings_ingested':False,
    'suitability_judgments_ingested':False,
    'mirror_graph_wiring':False,
  },
  'fields':['areasymbol','areaname','saversion','saverest'],
  'survey_areas':rows,
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
content='# stdlib/lib/corpus/usda-ssurgo-sacatalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usda-ssurgo-sacatalog.sh && scripts/gen-science-usda-ssurgo-sacatalog.sh\n'
content+='# 범위: SSURGO sacatalog survey-area metadata only. FGDC XML/prose·soil property tables·geometry·ratings 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: survey_areas={len(rows)} bytes={len(content.encode())}')
PY
