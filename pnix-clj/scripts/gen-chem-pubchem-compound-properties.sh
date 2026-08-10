#!/usr/bin/env bash
# PubChem PUG REST compound property JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${PUBCHEM_PROPERTIES_SRC:-$ROOT/ingest/chem/pubchem-compound-properties/compound-properties.json}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/pubchem-compound-properties.generated.px}"
RECEIPT="$ROOT/ingest/chem/pubchem-compound-properties/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing PubChem property JSON: $SRC" >&2
  echo "run scripts/update-chem-pubchem-compound-properties.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import hashlib, json, pathlib, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:])
raw=src.read_bytes(); obj=json.loads(raw.decode('utf-8'))
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
rows=[]
for r in (obj.get('PropertyTable') or {}).get('Properties') or []:
    rows.append({k:r.get(k) for k in sorted(r.keys()) if r.get(k) not in (None,'')})
rows=sorted(rows,key=lambda r:r.get('CID') or 0)
formula_counts={}
for r in rows:
    f=r.get('MolecularFormula') or 'unknown'
    formula_counts[f]=formula_counts.get(f,0)+1
out_obj={
 'schema':'chem.pubchem_compound_properties.v1',
 'source':{
   'name':'PubChem PUG REST compound property table',
   'license':'NIH/NCBI public domain core data with PubChem depositor/BioAssay caveat',
   'source_urls':['https://pubchem.ncbi.nlm.nih.gov/docs/pug-rest','https://pubchem.ncbi.nlm.nih.gov/docs/','https://www.ncbi.nlm.nih.gov/home/about/policies/'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-chem-pubchem-compound-properties.sh',
   'scope':'bounded compound property rows only; synonyms, Substance/SID depositor records, BioAssay/AID, PUG-View prose, hazard/handling/synthesis guidance excluded'
 },
 'source_files':{'compound_properties_json_sha256':hashlib.sha256(raw).hexdigest()},
 'summary':{
   'compound_count':len(rows),
   'cid_start':receipt.get('cid_start'),
   'cid_end':receipt.get('cid_end'),
   'requested_cid_count':receipt.get('requested_cid_count'),
   'properties':receipt.get('properties') or [],
   'synonyms_ingested':False,
   'substance_depositor_records_ingested':False,
   'bioassay_records_ingested':False,
   'pug_view_annotations_ingested':False,
   'hazard_or_handling_guidance_ingested':False,
   'synthesis_or_experimental_procedures_ingested':False,
   'mirror_graph_wiring':False,
 },
 'compounds':rows,
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
content='# stdlib/lib/corpus/pubchem-compound-properties.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-chem-pubchem-compound-properties.sh && scripts/gen-chem-pubchem-compound-properties.sh\n'
content+='# 범위: PubChem Compound property rows only. synonyms/SID/AID/PUG-View/hazard/synthesis guidance 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: compounds={len(rows)} bytes={len(content.encode())}')
PY
