#!/usr/bin/env bash
# eCFR 49 CFR 172.101 XML -> chunked pnix attrset source for UN/NA identifier rows only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ECFR_HAZMAT_XML_SRC:-$ROOT/ingest/regulatory/ecfr-49-172-101-hazmat-identifiers/title-49-172-101.xml}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/ecfr-49-172-101-hazmat-identifiers.generated.px}"
RECEIPT="$ROOT/ingest/regulatory/ecfr-49-172-101-hazmat-identifiers/source-receipt.json"
CHUNK_SIZE=500
CHUNK_INDEX=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --chunk-size) CHUNK_SIZE="$2"; shift 2 ;;
    --chunk-index) CHUNK_INDEX="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done
if [[ ! -f "$SRC" ]]; then
  echo "missing eCFR hazmat XML: $SRC" >&2
  echo "run scripts/update-regulatory-ecfr-hazmat-identifiers.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" "$CHUNK_SIZE" "$CHUNK_INDEX" <<'PY'
import hashlib, json, pathlib, re, sys, xml.etree.ElementTree as ET
src, out, receipt_path = map(pathlib.Path, sys.argv[1:4])
chunk_size=int(sys.argv[4]); chunk_index=int(sys.argv[5])
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
def txt(el):
    return ' '.join(''.join(el.itertext()).split())
def clean(v):
    v=' '.join((v or '').replace('\u00a0',' ').split())
    return None if v == '' else v
root=ET.parse(src).getroot()
tables=root.findall('.//TABLE')
hazmat=max(tables, key=lambda t: len(t.findall('.//TR')))
all_table_rows=hazmat.findall('.//TR')
records=[]
for tr in all_table_rows[2:]:
    cells=[txt(c) for c in list(tr) if c.tag == 'TD']
    if len(cells) < 4:
        continue
    ident=clean(cells[3])
    if not ident or not re.match(r'^(UN|NA)\d{4}$', ident):
        continue
    records.append({
      'symbol':clean(cells[0]),
      'proper_shipping_name':clean(cells[1]),
      'hazard_class_or_division':clean(cells[2]),
      'identification_number':ident,
      'regulatory_citation':'49 CFR 172.101',
    })
records=sorted(records,key=lambda r:(r.get('identification_number') or '', r.get('proper_shipping_name') or ''))
start=chunk_index*chunk_size; end=min(start+chunk_size,len(records)); chunk_rows=records[start:end]
by_prefix={}; by_class={}
for r in chunk_rows:
    p=(r.get('identification_number') or '')[:2] or 'unknown'
    by_prefix[p]=by_prefix.get(p,0)+1
    c=r.get('hazard_class_or_division') or 'unknown'
    by_class[c]=by_class.get(c,0)+1
obj={
 'schema':'regulatory.ecfr_49_172_101_hazmat_identifiers.v1',
 'source':{
   'name':'eCFR 49 CFR 172.101 Hazardous Materials Table identifier rows',
   'license':'US Government public domain / eCFR XML no downstream copyright restriction',
   'source_urls':['https://www.ecfr.gov/current/title-49/subtitle-B/chapter-I/subchapter-C/part-172/subpart-B/section-172.101','https://www.ecfr.gov/api/versioner/v1/titles.json','https://www.ecfr.gov/reader-aids/ecfr-developer-resources'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-regulatory-ecfr-hazmat-identifiers.sh',
   'scope':'UN/NA identifier metadata only; packaging, quantity limits, stowage, special provisions, RQ appendices, emergency response, compliance/legal advice excluded'
 },
 'source_files':{'ecfr_section_xml_sha256':hashlib.sha256(src.read_bytes()).hexdigest()},
 'summary':{
   'hazmat_table_tr_count':len(all_table_rows),
   'identifier_row_count_total':len(records),
   'chunk_index':chunk_index,
   'chunk_size':chunk_size,
   'chunk_start':start,
   'chunk_end':end,
   'stored_row_count':len(chunk_rows),
   'chunk_count_total':(len(records)+chunk_size-1)//chunk_size,
   'id_prefix_counts_chunk':[{'id_prefix': k, 'count': by_prefix[k]} for k in sorted(by_prefix)],
   'hazard_class_counts_chunk':[{'hazard_class_or_division': k, 'count': by_class[k]} for k in sorted(by_class)],
   'packaging_columns_ingested':False,
   'quantity_limit_columns_ingested':False,
   'stowage_columns_ingested':False,
   'special_provisions_ingested':False,
   'rq_appendix_tables_ingested':False,
   'emergency_response_or_compliance_advice_ingested':False,
   'mirror_graph_wiring':False,
 },
 'fields':['symbol','proper_shipping_name','hazard_class_or_division','identification_number','regulatory_citation'],
 'rows':chunk_rows,
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
content='# stdlib/lib/corpus/ecfr-49-172-101-hazmat-identifiers.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-regulatory-ecfr-hazmat-identifiers.sh && scripts/gen-regulatory-ecfr-hazmat-identifiers.sh\n'
content+='# 범위: 49 CFR 172.101 Hazardous Materials Table UN/NA identifier rows only. packaging/quantity/stowage/special provisions/RQ/compliance advice 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: chunk={chunk_index} rows={len(chunk_rows)}/{len(records)} chunks={(len(records)+chunk_size-1)//chunk_size} bytes={len(content.encode())}')
PY
