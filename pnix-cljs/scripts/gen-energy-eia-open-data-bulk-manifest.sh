#!/usr/bin/env bash
# EIA Open Data bulk manifest JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${EIA_BULK_MANIFEST_SRC:-$ROOT/ingest/energy/eia-open-data-bulk-manifest/manifest.txt}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/eia-open-data-bulk-manifest.generated.px}"
RECEIPT="$ROOT/ingest/energy/eia-open-data-bulk-manifest/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing EIA bulk manifest: $SRC" >&2
  echo "run scripts/update-energy-eia-open-data-bulk-manifest.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import hashlib, json, pathlib, re, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:])
raw=src.read_bytes(); obj=json.loads(raw.decode('utf-8'))
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
def clean(v):
    if v in (None,''): return None
    if isinstance(v,str): return v.strip() or None
    return v
def split_labels(v):
    v=clean(v)
    if not v: return []
    return [x.strip() for x in re.split(r',\s*', v) if x.strip()]
records=[]; temporal_counts={}; spatial_counts={}; format_counts={}
for ident, d in sorted((obj.get('dataset') or {}).items()):
    if not isinstance(d,dict): continue
    desc=clean(d.get('description'))
    keyword=clean(d.get('keyword'))
    temporal=split_labels(d.get('temporal'))
    spatial=split_labels(d.get('spatial'))
    fmt=clean(d.get('format'))
    rec={
      'identifier':clean(d.get('identifier') or ident),
      'data_set':clean(d.get('data_set')),
      'name':clean(d.get('name')),
      'title':clean(d.get('title')),
      'category_id':clean(d.get('category_id')),
      'last_updated':clean(d.get('last_updated')),
      'modified':clean(d.get('modified')),
      'publisher':clean(d.get('publisher')),
      'access_level':clean(d.get('accessLevel')),
      'access_url':clean(d.get('accessURL')),
      'web_service':clean(d.get('webService')),
      'format':fmt,
      'temporal_granularity':temporal,
      'spatial_granularity':spatial,
      'contact_email':clean(d.get('mbox')),
      'description_sha256':hashlib.sha256(desc.encode('utf-8')).hexdigest() if desc else None,
      'description_ingested':False,
      'keyword_sha256':hashlib.sha256(keyword.encode('utf-8')).hexdigest() if keyword else None,
      'keyword_text_ingested':False,
      'bulk_zip_payload_ingested':False,
      'time_series_values_ingested':False,
    }
    records.append(rec)
    for t in temporal or ['unknown']: temporal_counts[t]=temporal_counts.get(t,0)+1
    for s in spatial or ['unknown']: spatial_counts[s]=spatial_counts.get(s,0)+1
    if fmt: format_counts[fmt]=format_counts.get(fmt,0)+1

def pairs(d,k): return [{k:x,'count':d[x]} for x in sorted(d)]
out_obj={
 'schema':'energy.eia_open_data_bulk_manifest.v1',
 'source':{
   'name':'EIA Open Data bulk manifest',
   'license':'EIA public domain / acknowledgment requested',
   'source_urls':['https://www.eia.gov/opendata/','https://www.eia.gov/opendata/bulkfiles.php','https://api.eia.gov/bulk/manifest.txt','https://www.eia.gov/about/copyrights_reuse.php'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-energy-eia-open-data-bulk-manifest.sh',
   'scope':'bulk manifest dataset metadata only; bulk zip payloads, time-series values, operational dispatch guidance, security-sensitive facility detail, forecast/trading advice, and graph/mirror wiring excluded'
 },
 'source_files':{'manifest_txt_sha256':hashlib.sha256(raw).hexdigest()},
 'summary':{
   'dataset_count':len(records),
   'temporal_granularity_counts':pairs(temporal_counts,'temporal_granularity'),
   'spatial_granularity_counts':pairs(spatial_counts,'spatial_granularity'),
   'format_counts':pairs(format_counts,'format'),
   'bulk_zip_payloads_ingested':False,
   'time_series_values_ingested':False,
   'operational_dispatch_guidance_ingested':False,
   'facility_security_sensitive_payload_ingested':False,
   'forecast_or_trading_advice_ingested':False,
   'mirror_graph_wiring':False,
 },
 'datasets':records,
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
content='# stdlib/lib/corpus/eia-open-data-bulk-manifest.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-energy-eia-open-data-bulk-manifest.sh && scripts/gen-energy-eia-open-data-bulk-manifest.sh\n'
content+='# 범위: EIA Open Data bulk manifest metadata only. bulk zip/time-series/operation guidance 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: datasets={len(records)} bytes={len(content.encode())}')
PY
