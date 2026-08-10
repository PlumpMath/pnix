#!/usr/bin/env bash
# NIST CAD-PMI-Testing browser HTML -> pnix attrset source catalog.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NIST_CAD_PMI_CATALOG_SRC:-$ROOT/ingest/cad/nist-pmi-testcase-catalog/models.html}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/nist-cad-pmi-testcase-catalog.generated.px}"
RECEIPT="$ROOT/ingest/cad/nist-pmi-testcase-catalog/source-receipt.json"
if [[ ! -f "$SRC" ]]; then
  echo "missing NIST CAD PMI catalog HTML: $SRC" >&2
  echo "run scripts/update-cad-nist-pmi-testcase-catalog.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import hashlib, html.parser, json, pathlib, re, sys
src, out, receipt_path = map(pathlib.Path, sys.argv[1:])
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
text=src.read_text(encoding='utf-8',errors='replace')
class P(html.parser.HTMLParser):
    def __init__(self): super().__init__(); self.links=[]; self.in_a=False; self.href=None; self.label=''
    def handle_starttag(self, tag, attrs):
        if tag=='a':
            self.in_a=True; self.href=dict(attrs).get('href'); self.label=''
    def handle_data(self,d):
        if self.in_a: self.label += d
    def handle_endtag(self, tag):
        if tag=='a' and self.in_a:
            label=' '.join(self.label.split())
            if self.href and re.match(r'^https://www\.nist\.gov/document/nist-cad-model-(ctc|ftc)-\d+', self.href):
                self.links.append({'label':label,'url':self.href})
            self.in_a=False; self.href=None; self.label=''
p=P(); p.feed(text)
items=[]
for x in p.links:
    m=re.search(r'nist-cad-model-(ctc|ftc)-(\d+)', x['url'])
    fam=m.group(1).upper() if m else None
    num=int(m.group(2)) if m else None
    items.append({'family':fam,'case_number':num,'label':x['label'],'nist_document_url':x['url'],'payload_downloaded':False})
items=sorted(items,key=lambda r:(r['family'] or '', r['case_number'] or 0))
by_family={}
for r in items: by_family[r['family'] or 'unknown']=by_family.get(r['family'] or 'unknown',0)+1
obj={
 'schema':'cad.nist_pmi_testcase_catalog.v1',
 'source':{
   'name':'NIST CAD Models and STEP Files with PMI / CAD-PMI-Testing test case browser',
   'license':'NIST public data / public domain where applicable',
   'source_urls':['https://pages.nist.gov/CAD-PMI-Testing/models.html','https://www.nist.gov/ctl/smart-connected-systems-division/smart-connected-manufacturing-systems-group/mbe-pmi-0','https://www.nist.gov/open/license'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-cad-nist-pmi-testcase-catalog.sh',
   'scope':'test-case catalog metadata only; CAD/STEP/native model/drawing/PMI body/report payloads and process/toolpath guidance excluded'
 },
 'source_files':{'models_html_sha256':hashlib.sha256(src.read_bytes()).hexdigest()},
 'summary':{
   'test_case_count':len(items),
   'family_counts':[{'family':k,'count':by_family[k]} for k in sorted(by_family)],
   'cad_files_downloaded':False,
   'step_files_downloaded':False,
   'drawing_or_pdf_bodies_ingested':False,
   'pmi_detail_bodies_ingested':False,
   'reports_ingested':False,
   'toolpath_or_process_guidance_ingested':False,
   'mirror_graph_wiring':False,
 },
 'fields':['family','case_number','label','nist_document_url','payload_downloaded'],
 'test_cases':items,
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
content='# stdlib/lib/corpus/nist-cad-pmi-testcase-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-cad-nist-pmi-testcase-catalog.sh && scripts/gen-cad-nist-pmi-testcase-catalog.sh\n'
content+='# 범위: NIST CAD PMI test-case catalog metadata only. CAD/STEP/PDF/PMI bodies/process guidance 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: test_cases={len(items)} bytes={len(content.encode())}')
PY
