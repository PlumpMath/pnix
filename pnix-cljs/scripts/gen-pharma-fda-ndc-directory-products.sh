#!/usr/bin/env bash
# openFDA NDC Directory JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NDC_SRC:-$ROOT/ingest/pharma/fda-ndc-directory-products/ndc.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/fda-ndc-directory-products.generated.px}"
RECEIPT="$ROOT/ingest/pharma/fda-ndc-directory-products/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing NDC JSON: $SRC" >&2
  echo "run scripts/update-pharma-fda-ndc-directory-products.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import hashlib, json, pathlib, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:])
raw=src.read_bytes(); obj=json.loads(raw.decode('utf-8'))
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
def clean(v):
    if v in (None,''): return None
    if isinstance(v,str): return v.strip() or None
    return v
def uniq(xs):
    out=[]
    for x in xs:
        x=clean(x)
        if x and x not in out: out.append(x)
    return out
products=[]; package_count=0; route_counts={}; category_counts={}; type_counts={}; form_counts={}
for r in obj.get('results') or []:
    ingredients=uniq([ai.get('name') for ai in (r.get('active_ingredients') or []) if isinstance(ai,dict)])
    packages=[]
    for p in r.get('packaging') or []:
        if not isinstance(p,dict): continue
        desc=clean(p.get('description'))
        packages.append({
          'package_ndc':clean(p.get('package_ndc')),
          'marketing_start_date':clean(p.get('marketing_start_date')),
          'sample':clean(p.get('sample')),
          'package_description_sha256':hashlib.sha256(desc.encode('utf-8')).hexdigest() if desc else None,
          'package_description_ingested':False,
        })
        package_count += 1
    routes=uniq(r.get('route') or []) if isinstance(r.get('route'),list) else uniq([r.get('route')])
    rec={
      'product_ndc':clean(r.get('product_ndc')),
      'product_id':clean(r.get('product_id')),
      'application_number':clean(r.get('application_number')),
      'labeler_name':clean(r.get('labeler_name')),
      'brand_name':clean(r.get('brand_name')),
      'brand_name_base':clean(r.get('brand_name_base')),
      'brand_name_suffix':clean(r.get('brand_name_suffix')),
      'generic_name':clean(r.get('generic_name')),
      'product_type':clean(r.get('product_type')),
      'marketing_category':clean(r.get('marketing_category')),
      'dosage_form':clean(r.get('dosage_form')),
      'route':routes,
      'finished':clean(r.get('finished')),
      'marketing_start_date':clean(r.get('marketing_start_date')),
      'listing_expiration_date':clean(r.get('listing_expiration_date')),
      'spl_id':clean(r.get('spl_id')),
      'active_ingredient_names':ingredients,
      'active_ingredient_strength_values_ingested':False,
      'pharm_class':uniq(r.get('pharm_class') or []) if isinstance(r.get('pharm_class'),list) else uniq([r.get('pharm_class')]),
      'packages':sorted(packages,key=lambda x:x.get('package_ndc') or ''),
      'package_description_text_ingested':False,
      'openfda_crosswalk_ingested':False,
      'label_text_ingested':False,
      'prescribing_or_safety_guidance_ingested':False,
    }
    products.append(rec)
    for route in routes or ['unknown']: route_counts[route]=route_counts.get(route,0)+1
    category_counts[rec.get('marketing_category') or 'unknown']=category_counts.get(rec.get('marketing_category') or 'unknown',0)+1
    type_counts[rec.get('product_type') or 'unknown']=type_counts.get(rec.get('product_type') or 'unknown',0)+1
    form_counts[rec.get('dosage_form') or 'unknown']=form_counts.get(rec.get('dosage_form') or 'unknown',0)+1
products=sorted(products,key=lambda x:x.get('product_ndc') or '')
def count_pairs(d, key_name): return [{key_name:k,'count':d[k]} for k in sorted(d)]
out_obj={
 'schema':'pharma.fda_ndc_directory_products.v1',
 'source':{
   'name':'openFDA Drug NDC Directory product/package metadata',
   'license':'openFDA public domain / CC0 public data policy',
   'source_urls':['https://api.fda.gov/download.json','https://api.fda.gov/drug/ndc.json','https://open.fda.gov/apis/drug/ndc/','https://open.fda.gov/license/'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-pharma-fda-ndc-directory-products.sh',
   'scope':'bounded product/package identifier metadata only; label text, active ingredient strength values, package description prose, prescribing/safety guidance, adverse-event/enforcement payloads, and graph/mirror wiring excluded'
 },
 'source_files':{'ndc_json_sha256':hashlib.sha256(raw).hexdigest()},
 'summary':{
   'product_count':len(products),
   'package_count':package_count,
   'api_total_records':receipt.get('api_total_records'),
   'openfda_total_records':receipt.get('openfda_total_records'),
   'openfda_export_date':receipt.get('openfda_export_date'),
   'limit':receipt.get('limit'),
   'skip':receipt.get('skip'),
   'route_counts':count_pairs(route_counts,'route'),
   'marketing_category_counts':count_pairs(category_counts,'marketing_category'),
   'product_type_counts':count_pairs(type_counts,'product_type'),
   'dosage_form_counts':count_pairs(form_counts,'dosage_form'),
   'labeling_text_ingested':False,
   'active_ingredient_strength_values_ingested':False,
   'package_description_text_ingested':False,
   'prescribing_or_safety_guidance_ingested':False,
   'adverse_event_or_enforcement_data_ingested':False,
   'mirror_graph_wiring':False,
 },
 'products':products,
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
content='# stdlib/lib/corpus/fda-ndc-directory-products.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-pharma-fda-ndc-directory-products.sh && scripts/gen-pharma-fda-ndc-directory-products.sh\n'
content+='# 범위: NDC product/package identifier metadata only. labels/strength/package prose/prescribing/safety 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: products={len(products)} packages={package_count} bytes={len(content.encode())}')
PY
