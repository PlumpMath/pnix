#!/usr/bin/env bash
# FDA Purple Book CSV -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${PURPLEBOOK_SRC:-$ROOT/ingest/pharma/fda-purplebook-products/purplebook.csv}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/fda-purplebook-products.generated.px}"
RECEIPT="$ROOT/ingest/pharma/fda-purplebook-products/source-receipt.json"
LIMIT="${PURPLEBOOK_LIMIT:-250}"
CHANGES_LIMIT="${PURPLEBOOK_CHANGES_LIMIT:-100}"
if [[ ! -f "$SRC" ]]; then
  echo "missing Purple Book CSV: $SRC" >&2
  echo "run scripts/update-pharma-fda-purplebook-products.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" "$LIMIT" "$CHANGES_LIMIT" <<'PY'
import csv, hashlib, io, json, pathlib, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:4]); limit=int(sys.argv[4]); changes_limit=int(sys.argv[5])
raw=src.read_bytes(); text=raw.decode('utf-8-sig','replace')
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
def clean(v):
    if v is None: return None
    v=str(v).strip()
    if v in ('','N/A','n/a'): return None
    return v
def norm_header(h): return (h or '').strip()
rows=list(csv.reader(io.StringIO(text)))
sections=[]; i=0
while i < len(rows):
    row=rows[i]
    if row and norm_header(row[0]) == 'N/R/U' and any(norm_header(c)=='BLA Number' for c in row):
        header=[norm_header(c) for c in row]
        data=[]; i += 1
        while i < len(rows):
            r=rows[i]
            if r and norm_header(r[0]) == 'N/R/U' and any(norm_header(c)=='BLA Number' for c in r):
                i -= 1; break
            if any(clean(c) for c in r): data.append(r)
            i += 1
        sections.append((header,data))
    i += 1
changes_header, changes_rows = sections[0] if sections else ([],[])
all_header, all_rows = sections[1] if len(sections) > 1 else (changes_header, changes_rows)
def rowdict(header,row):
    return {header[j]: clean(row[j]) if j < len(row) else None for j in range(len(header))}
def product_rec(d, section):
    return {
      'section':section,
      'change_code':d.get('N/R/U'),
      'applicant':d.get('Applicant'),
      'bla_number':d.get('BLA Number'),
      'proprietary_name':d.get('Proprietary Name'),
      'proper_name':d.get('Proper Name'),
      'license_type':d.get('License Type'),
      'dosage_form':d.get('Dosage Form'),
      'route_of_administration':d.get('Route of Administration'),
      'product_presentation':d.get('Product Presentation'),
      'marketing_status':d.get('Marketing Status'),
      'licensure':d.get('Licensure'),
      'approval_date':d.get('Approval Date'),
      'interchangeable_approval_date':d.get('Inter. Approval Date'),
      'reference_product_proper_name':d.get('Ref. Product Proper Name'),
      'reference_product_proprietary_name':d.get('Ref. Product Proprietary Name'),
      'supplement_number':d.get('Supplement Number'),
      'submission_type':d.get('Submission Type'),
      'interchangeable_supplement_number':d.get('Inter. Supplement Number'),
      'license_number':d.get('License Number'),
      'product_number':d.get('Product Number'),
      'center':d.get('Center'),
      'date_of_first_licensure':d.get('Date of First Licensure'),
      'strength_values_ingested':False,
      'label_text_ingested':False,
      'patent_detail_payload_ingested':False,
      'clinical_use_guidance_ingested':False,
    }
all_products=[product_rec(rowdict(all_header,r),'all_products') for r in all_rows]
changes=[product_rec(rowdict(changes_header,r),'monthly_changes') for r in changes_rows]
all_total=len(all_products); changes_total=len(changes)
all_products=all_products[:limit]
changes=changes[:changes_limit]
def counts(records,key):
    d={}
    for r in records:
        v=r.get(key) or 'unknown'; d[v]=d.get(v,0)+1
    return [{'value':k,'count':d[k]} for k in sorted(d)]
out_obj={
 'schema':'pharma.fda_purplebook_products.v1',
 'source':{
   'name':'FDA Purple Book monthly downloadable product data',
   'license':'FDA public data / US federal public information',
   'source_urls':['https://purplebooksearch.fda.gov/downloads','https://purplebooksearch.fda.gov/userguide'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-pharma-fda-purplebook-products.sh',
   'scope':'bounded biologic product identifier/relationship metadata only; strength values, label text, prescribing/safety guidance, patent details, clinical-use interpretation, and graph/mirror wiring excluded'
 },
 'source_files':{'purplebook_csv_sha256':hashlib.sha256(raw).hexdigest()},
 'summary':{
   'all_products_total_available':all_total,
   'all_products_count':len(all_products),
   'monthly_changes_total_available':changes_total,
   'monthly_changes_count':len(changes),
   'limit':limit,
   'changes_limit':changes_limit,
   'selected_year':receipt.get('selected_year'),
   'selected_month':receipt.get('selected_month'),
   'license_type_counts':counts(all_products,'license_type'),
   'center_counts':counts(all_products,'center'),
   'marketing_status_counts':counts(all_products,'marketing_status'),
   'licensure_counts':counts(all_products,'licensure'),
   'strength_values_ingested':False,
   'labeling_text_ingested':False,
   'prescribing_or_safety_guidance_ingested':False,
   'patent_detail_payload_ingested':False,
   'clinical_use_interpretation_ingested':False,
   'mirror_graph_wiring':False,
 },
 'products':all_products,
 'monthly_changes':changes,
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
content='# stdlib/lib/corpus/fda-purplebook-products.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-pharma-fda-purplebook-products.sh && scripts/gen-pharma-fda-purplebook-products.sh\n'
content+='# 범위: Purple Book biologic product identifier/relationship metadata only. strength/label/prescribing/safety/patent-detail 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: products={len(all_products)}/{all_total} changes={len(changes)}/{changes_total} bytes={len(content.encode())}')
PY
