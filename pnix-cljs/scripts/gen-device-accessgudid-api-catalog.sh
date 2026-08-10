#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/device/accessgudid-api-catalog"
OUT="$ROOT/stdlib/lib/corpus/accessgudid-api-catalog.generated.px"
python3 - "$IN" "$OUT" <<'PY'
import html,re,sys
from pathlib import Path
root=Path(sys.argv[1]); out=Path(sys.argv[2])

def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
pages=[]; fields=[]; endpoints=[]
for f in sorted(root.glob('*.html')):
    raw=f.read_text(errors='ignore')
    title=re.search(r'<title>(.*?)</title>', raw, re.S)
    page={'page':f.stem,'title':html.unescape(re.sub('<[^>]+>',' ',title.group(1))).strip() if title else ''}
    pages.append(page)
    for u in sorted(set(re.findall(r'https://accessgudid\.nlm\.nih\.gov/api/v[0-9]+/[^<\s]+', raw))):
        endpoints.append({'page':f.stem,'url':html.unescape(u).strip()})
    for tr in re.findall(r'<tr>(.*?)</tr>', raw, re.S):
        cells=[html.unescape(re.sub(r'<[^>]+>',' ',c)).strip() for c in re.findall(r'<td[^>]*>(.*?)</td>', tr, re.S)]
        cells=[' '.join(c.split()) for c in cells]
        if len(cells)>=2 and re.match(r'^[A-Za-z_][A-Za-z0-9_]*$', cells[0]):
            fields.append({'page':f.stem,'name':cells[0],'type':cells[1]})
seed={'schema':'device.accessgudid_api_catalog.v1','source':{'name':'AccessGUDID developer API documentation','license':'U.S. federal public information','base':'https://accessgudid.nlm.nih.gov/'},'summary':{'page_count':len(pages),'endpoint_count':len(endpoints),'field_count':len(fields),'device_records_ingested':False,'udi_di_values_ingested':False,'full_downloads_ingested':False,'documentation_prose_ingested':False,'medical_advice_ingested':False,'mirror_graph_wiring':False},'pages':pages,'endpoints':endpoints,'fields':fields}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: pages={len(pages)} endpoints={len(endpoints)} fields={len(fields)}')
PY
