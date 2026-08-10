#!/usr/bin/env bash
# WIPO Locarno Classification class/subclass headings -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${LOCARNO_SRC:-$ROOT/ingest/design/wipo-locarno-classification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/wipo-locarno-classification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing WIPO Locarno snapshot: $SRC" >&2
  echo "run scripts/update-design-wipo-locarno-classification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import html, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
raw=(src/'raw'/'class_subclass_headings.html').read_text(encoding='utf-8', errors='replace')
classes=[]; subclasses=[]
class_pat=re.compile(r'Class\s+(\d+)\s*</h1>\s*<span[^>]*>(.*?)</span>(.*?)(?=<div[^>]+class="class_heading"|</body>|$)', re.S)
sub_pat=re.compile(r'<span[^>]*>\s*(\d{2}-\d{2})\s+([^<]+?)\s*</span>', re.S)
for cm in class_pat.finditer(raw):
    cnum=int(cm.group(1)); title=html.unescape(re.sub(r'<[^>]+>|\s+',' ',cm.group(2))).strip()
    body=cm.group(3)
    classes.append({'class_number':cnum,'title':title})
    for sm in sub_pat.finditer(body):
        code=sm.group(1); label=html.unescape(re.sub(r'\s+',' ',sm.group(2))).strip()
        subclasses.append({'class_number':cnum,'subclass_code':code,'title':label})
classes=sorted({c['class_number']:c for c in classes}.values(), key=lambda r:r['class_number'])
seen=set(); uniq=[]
for row in subclasses:
    key=(row['class_number'],row['subclass_code'])
    if key not in seen:
        seen.add(key); uniq.append(row)
subclasses=sorted(uniq, key=lambda r:(r['class_number'],r['subclass_code']))
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')} for f in receipt.get('files') or []]
obj={'schema':'design.wipo_locarno.classification.v1','source':{'name':'WIPO Locarno Classification class/subclass headings','license':'WIPO official downloadable classification data / WIPO attribution family','source_urls':['https://www.wipo.int/en/web/classification-locarno','https://www.wipo.int/classifications/locarno/en/ITsupport/','https://locpub.wipo.int/enfr/'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'version':receipt.get('version'),'lang':receipt.get('lang'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-design-wipo-locarno-classification.sh','scope':'official class/subclass headings only; detailed terms/prose/legal guidance excluded'},'summary':{'version':receipt.get('version'),'lang':receipt.get('lang'),'class_count':len(classes),'subclass_count':len(subclasses),'alphabetical_terms_ingested':False,'explanatory_notes_ingested':False,'general_remarks_ingested':False,'legal_guidance_ingested':False,'design_image_payloads_ingested':False,'design_application_payloads_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'classes':classes,'subclasses':subclasses}
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
content='# stdlib/lib/corpus/wipo-locarno-classification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-design-wipo-locarno-classification.sh && scripts/gen-design-wipo-locarno-classification.sh\n'
content+='# 범위: WIPO Locarno class/subclass headings only. terms/prose/legal guidance/images/application payload/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: classes={len(classes)} subclasses={len(subclasses)} bytes={len(content.encode())}')
PY
