#!/usr/bin/env bash
# openFDA Drugs@FDA JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${DRUGSFDA_SRC:-$ROOT/ingest/pharma/fda-drugsfda-products/drugsfda.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/fda-drugsfda-products.generated.px}"
RECEIPT="$ROOT/ingest/pharma/fda-drugsfda-products/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing Drugs@FDA JSON: $SRC" >&2
  echo "run scripts/update-pharma-fda-drugsfda-products.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import hashlib, json, pathlib, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:])
raw=src.read_bytes(); obj=json.loads(raw.decode('utf-8'))
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
def clean(v): return None if v in (None,'') else v
apps=[]; product_count=0; status_counts={}; route_counts={}
for app in obj.get('results') or []:
    products=[]
    for p in app.get('products') or []:
        ingredient_names=[]
        for ai in p.get('active_ingredients') or []:
            n=clean(ai.get('name'))
            if n and n not in ingredient_names: ingredient_names.append(n)
        rec={
          'product_number':clean(p.get('product_number')),
          'brand_name':clean(p.get('brand_name')),
          'active_ingredient_names':ingredient_names,
          'reference_drug':clean(p.get('reference_drug')),
          'reference_standard':clean(p.get('reference_standard')),
          'dosage_form':clean(p.get('dosage_form')),
          'route':clean(p.get('route')),
          'marketing_status':clean(p.get('marketing_status')),
          'strength_values_ingested':False,
        }
        products.append(rec); product_count += 1
        status_counts[rec.get('marketing_status') or 'unknown']=status_counts.get(rec.get('marketing_status') or 'unknown',0)+1
        route_counts[rec.get('route') or 'unknown']=route_counts.get(rec.get('route') or 'unknown',0)+1
    apps.append({
      'application_number':clean(app.get('application_number')),
      'sponsor_name':clean(app.get('sponsor_name')),
      'product_count':len(products),
      'products':sorted(products,key=lambda x:x.get('product_number') or ''),
      'submissions_ingested':False,
      'openfda_crosswalk_ingested':False,
    })
apps=sorted(apps,key=lambda x:x.get('application_number') or '')
out_obj={
 'schema':'pharma.fda_drugsfda_products.v1',
 'source':{
   'name':'openFDA Drugs@FDA application/product metadata',
   'license':'openFDA public domain / public data policy',
   'source_urls':['https://api.fda.gov/download.json','https://api.fda.gov/drug/drugsfda.json','https://open.fda.gov/apis/drug/drugsfda/','https://open.fda.gov/license/'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-pharma-fda-drugsfda-products.sh',
   'scope':'bounded application/product identifier metadata only; submissions/application_docs/labels/dosage-strength/prescribing/safety/adverse-event/recall payloads excluded'
 },
 'source_files':{'drugsfda_json_sha256':hashlib.sha256(raw).hexdigest()},
 'summary':{
   'application_count':len(apps),
   'product_count':product_count,
   'api_total_records':receipt.get('api_total_records'),
   'openfda_total_records':receipt.get('openfda_total_records'),
   'openfda_export_date':receipt.get('openfda_export_date'),
   'limit':receipt.get('limit'),
   'skip':receipt.get('skip'),
   'marketing_status_counts': [{'marketing_status':k,'count':status_counts[k]} for k in sorted(status_counts)],
   'route_counts': [{'route':k,'count':route_counts[k]} for k in sorted(route_counts)],
   'submissions_ingested':False,
   'application_docs_ingested':False,
   'labeling_text_ingested':False,
   'strength_values_ingested':False,
   'prescribing_or_safety_guidance_ingested':False,
   'adverse_event_or_enforcement_data_ingested':False,
   'mirror_graph_wiring':False,
 },
 'applications':apps,
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
content='# stdlib/lib/corpus/fda-drugsfda-products.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-pharma-fda-drugsfda-products.sh && scripts/gen-pharma-fda-drugsfda-products.sh\n'
content+='# 범위: Drugs@FDA application/product metadata only. submissions/docs/labels/strength/prescribing/safety 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: applications={len(apps)} products={product_count} bytes={len(content.encode())}')
PY
