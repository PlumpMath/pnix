#!/usr/bin/env bash
# WIPO Nice Classification class headings -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NICE_SRC:-$ROOT/ingest/trademark/wipo-nice-classification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/wipo-nice-classification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing WIPO Nice snapshot: $SRC" >&2
  echo "run scripts/update-trademark-wipo-nice-classification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import html, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
raw=(src/'raw'/'class_headings.html').read_text(encoding='utf-8', errors='replace')
rows=[]
pattern=re.compile(r'Class\s+(\d+)\s*<!-- end show label -->\s*</h1>\s*<!-- begin show classHeading -->(.*?)</div>', re.S)
for m in pattern.finditer(raw):
    cls=int(m.group(1)); heading=html.unescape(re.sub(r'<[^>]+>|\s+',' ',m.group(2))).strip()
    if cls < 1 or cls > 45 or not heading: continue
    rows.append({'class_number':cls,'kind':'goods' if cls<=34 else 'services','heading':heading})
rows=sorted(rows, key=lambda r:r['class_number'])
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')} for f in receipt.get('files') or []]
obj={'schema':'trademark.wipo_nice.classification.v1','source':{'name':'WIPO Nice Classification NCLPUB class headings','license':'WIPO official downloadable classification data / WIPO attribution family','source_urls':['https://www.wipo.int/en/web/classification-nice','https://www.wipo.int/classifications/nice/en/ITsupport/','https://nclpub.wipo.int/enfr/'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'version':receipt.get('version'),'lang':receipt.get('lang'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-trademark-wipo-nice-classification.sh','scope':'official class headings only; detailed terms/prose/legal guidance excluded'},'summary':{'version':receipt.get('version'),'lang':receipt.get('lang'),'class_count':len(rows),'goods_class_count':sum(1 for r in rows if r['kind']=='goods'),'services_class_count':sum(1 for r in rows if r['kind']=='services'),'alphabetical_terms_ingested':False,'explanatory_notes_ingested':False,'general_remarks_ingested':False,'legal_guidance_ingested':False,'trademark_application_payloads_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'classes':rows}
def pnix(v,indent=0):
    import json
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v),ensure_ascii=False)
content='# stdlib/lib/corpus/wipo-nice-classification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-trademark-wipo-nice-classification.sh && scripts/gen-trademark-wipo-nice-classification.sh\n'
content+='# 범위: WIPO Nice class headings only. terms/prose/legal guidance/application payload/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: classes={len(rows)} bytes={len(content.encode())}')
PY
