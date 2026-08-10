#!/usr/bin/env bash
# USGS MRDS flattened CSV -> bounded pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${USGS_MRDS_ZIP_SRC:-$ROOT/ingest/science/usgs-mrds-compact/mrds-csv.zip}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-mrds-compact.generated.px}"
RECEIPT="$ROOT/ingest/science/usgs-mrds-compact/source-receipt.json"
LIMIT="${MRDS_LIMIT:-5000}"
COUNTRY="${MRDS_COUNTRY:-United States}"
if [[ ! -f "$SRC" ]]; then
  echo "missing MRDS CSV zip: $SRC" >&2
  echo "run scripts/update-science-usgs-mrds-compact.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" "$LIMIT" "$COUNTRY" <<'PY'
import csv, hashlib, json, pathlib, sys, zipfile
src, out, receipt_path = map(pathlib.Path, sys.argv[1:4])
limit=int(sys.argv[4]); country_filter=sys.argv[5]
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
if isinstance(receipt.get('zip_entries'), dict):
    receipt['zip_entries']=[{'name': k, 'size_bytes': receipt['zip_entries'][k]} for k in sorted(receipt['zip_entries'])]
def clean(v):
    return None if v is None or v == '' else v
def num(v):
    if v in (None,''): return None
    try: return float(v)
    except Exception: return None
fields=['dep_id','url','mrds_id','mas_id','site_name','latitude','longitude','region','country','state','county','com_type','commod1','commod2','commod3','oper_type','dep_type','dev_stat','score']
records=[]; total_rows=0; country_rows=0
with zipfile.ZipFile(src) as z:
    csv_name=next(n for n in z.namelist() if n.lower().endswith('.csv'))
    with z.open(csv_name) as f:
        text=(line.decode('utf-8','replace') for line in f)
        reader=csv.DictReader(text)
        for row in reader:
            total_rows += 1
            if country_filter and row.get('country') != country_filter:
                continue
            country_rows += 1
            if len(records) >= limit:
                continue
            records.append({
              'dep_id':clean(row.get('dep_id')),
              'url':clean(row.get('url')),
              'mrds_id':clean(row.get('mrds_id')),
              'mas_id':clean(row.get('mas_id')),
              'site_name':clean(row.get('site_name')),
              'latitude':num(row.get('latitude')),
              'longitude':num(row.get('longitude')),
              'region':clean(row.get('region')),
              'country':clean(row.get('country')),
              'state':clean(row.get('state')),
              'county':clean(row.get('county')),
              'com_type':clean(row.get('com_type')),
              'commod1':clean(row.get('commod1')),
              'commod2':clean(row.get('commod2')),
              'commod3':clean(row.get('commod3')),
              'oper_type':clean(row.get('oper_type')),
              'dep_type':clean(row.get('dep_type')),
              'dev_stat':clean(row.get('dev_stat')),
              'score':clean(row.get('score')),
            })
records=sorted(records,key=lambda r:r.get('dep_id') or '')
by_state={}; by_score={}; by_commodity={}
for r in records:
    by_state[r.get('state') or 'unknown']=by_state.get(r.get('state') or 'unknown',0)+1
    by_score[r.get('score') or 'unknown']=by_score.get(r.get('score') or 'unknown',0)+1
    c=(r.get('commod1') or 'unknown').split(';')[0].strip() or 'unknown'
    by_commodity[c]=by_commodity.get(c,0)+1
obj={
 'schema':'science.usgs.mrds_compact.v1',
 'source':{
   'name':'USGS Mineral Resources Data System (MRDS) flattened CSV',
   'license':'US Government public domain',
   'source_urls':['https://mrdata.usgs.gov/mrds/','https://mrdata.usgs.gov/mrds/mrds-csv.zip','https://mrdata.usgs.gov/mrds/about.php'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-usgs-mrds-compact.sh',
   'scope':'bounded flattened CSV metadata only; production/resource quantities, geologic prose, references, full reports, geometry packages, extraction guidance excluded'
 },
 'source_files':{'mrds_csv_zip_sha256':hashlib.sha256(src.read_bytes()).hexdigest()},
 'summary':{
   'total_csv_rows_seen':total_rows,
   'country_filter':country_filter,
   'country_rows_seen':country_rows,
   'stored_record_count':len(records),
   'limit':limit,
   'state_counts':by_state,
   'score_counts':by_score,
   'primary_commodity_counts_bounded':by_commodity,
   'production_resource_fields_ingested':False,
   'geologic_prose_ingested':False,
   'references_ingested':False,
   'geometry_package_ingested':False,
   'extraction_guidance_ingested':False,
   'mirror_graph_wiring':False,
 },
 'fields':fields,
 'records':records,
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
content='# stdlib/lib/corpus/usgs-mrds-compact.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-mrds-compact.sh && scripts/gen-science-usgs-mrds-compact.sh\n'
content+='# 범위: bounded MRDS feature metadata only. production/resource/geologic prose/refs/full reports/geometry/extraction guidance 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: records={len(records)} total_rows={total_rows} country_rows={country_rows} bytes={len(content.encode())}')
PY
