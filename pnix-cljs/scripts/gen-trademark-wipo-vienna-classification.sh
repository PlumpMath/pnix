#!/usr/bin/env bash
# WIPO Vienna Classification structure -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${VIENNA_SRC:-$ROOT/ingest/trademark/wipo-vienna-classification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/wipo-vienna-classification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing WIPO Vienna snapshot: $SRC" >&2
  echo "run scripts/update-trademark-wipo-vienna-classification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
root=ET.parse(src/'raw'/'full.xml').getroot()
categories=[]; divisions=[]; sections=[]
for cat in root.findall('category'):
    cid=cat.attrib.get('id'); ctext=cat.attrib.get('text')
    categories.append({'category':cid,'label':ctext})
    for div in cat.findall('division'):
        did=div.attrib.get('id'); dtext=div.attrib.get('text')
        divisions.append({'category':cid,'division':did,'code':f'{int(cid):02d}.{int(did):02d}' if cid and did and cid.isdigit() and did.isdigit() else None,'label':dtext})
        for sec in div.findall('section'):
            sid=sec.attrib.get('id'); stext=sec.attrib.get('text')
            sections.append({'category':cid,'division':did,'section':sid,'code':f'{int(cid):02d}.{int(did):02d}.{int(sid):02d}' if cid and did and sid and cid.isdigit() and did.isdigit() and sid.isdigit() else None,'label':stext})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')} for f in receipt.get('files') or []]
obj={'schema':'trademark.wipo_vienna.classification.v1','source':{'name':'WIPO Vienna Classification category/division/section metadata','license':'WIPO official downloadable classification data / WIPO attribution family','source_urls':['https://www.wipo.int/en/web/classification-vienna','https://www.wipo.int/classifications/vienna/en/ITsupport/','https://nivilo.wipo.int/vienna.htm'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'version':receipt.get('version'),'lang':receipt.get('lang'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-trademark-wipo-vienna-classification.sh','scope':'category/division/section identifiers and labels only; notes/prose/images/legal guidance excluded'},'summary':{'version':receipt.get('version'),'lang':receipt.get('lang'),'category_count':len(categories),'division_count':len(divisions),'section_count':len(sections),'explanatory_notes_ingested':False,'guidance_or_legal_prose_ingested':False,'image_logo_payloads_ingested':False,'trademark_application_payloads_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'categories':categories,'divisions':divisions,'sections':sections}
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
content='# stdlib/lib/corpus/wipo-vienna-classification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-trademark-wipo-vienna-classification.sh && scripts/gen-trademark-wipo-vienna-classification.sh\n'
content+='# 범위: WIPO Vienna category/division/section labels only. notes/prose/images/legal guidance/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: categories={len(categories)} divisions={len(divisions)} sections={len(sections)} bytes={len(content.encode())}')
PY
