#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NASA_JPL_CATALOG_SRC:-$ROOT/ingest/space/nasa-jpl-ssd-naif-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/nasa-jpl-ssd-naif-catalog.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then echo "missing NASA/JPL catalog snapshot: $SRC" >&2; exit 2; fi
python3 - "$SRC" "$OUT" <<'PY'
import html, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text())
pages=[]; endpoints=[]; dir_entries=[]
for f in receipt.get('files',[]):
    text=(src/f['relative_path']).read_text(encoding='utf-8',errors='ignore')
    title=''
    m=re.search(r'<title[^>]*>(.*?)</title>',text,re.I|re.S)
    if m: title=re.sub(r'\s+',' ',html.unescape(re.sub('<[^>]+>','',m.group(1)))).strip()[:160]
    page={'family':f['family'],'name':f['name'],'url':f['url'],'status':f.get('status'),'sha256':f['sha256'],'size_bytes':f['size_bytes'],'title':title}
    pages.append(page)
    if f['family']=='ssd':
        for m in re.finditer(r'https://ssd-api\.jpl\.nasa\.gov/([A-Za-z0-9_./-]+\.api)', text):
            endpoints.append({'page':f['name'],'endpoint':'https://ssd-api.jpl.nasa.gov/'+m.group(1)})
        for m in re.finditer(r'\b([A-Za-z][A-Za-z0-9_-]{1,40})\s*=', text):
            endpoints.append({'page':f['name'],'query_param_token':m.group(1)})
    if f['family']=='naif' and 'generic_' in f['name']:
        for m in re.finditer(r'href="([^"]+)"', text, re.I):
            href=html.unescape(m.group(1))
            if href.startswith('?') or href.startswith('/') or href.startswith('../'): continue
            if len(dir_entries)<300:
                dir_entries.append({'directory':f['name'],'href':href,'kind':'kernel_or_subdir_ref'})
obj={'schema':'space.nasa_jpl.ssd_naif_catalog.v1','source':{'name':'NASA/JPL SSD API and NAIF public catalog metadata','license':'NASA/JPL public documentation + NAIF redistribution-permitted public data metadata','source_urls':['https://ssd-api.jpl.nasa.gov/','https://naif.jpl.nasa.gov/naif/rules.html'],'receipt':receipt,'generator':'scripts/gen-space-nasa-jpl-ssd-naif-catalog.sh','scope':'catalog metadata only; API/kernel payloads, trajectories, hazard rows, execution, and graph wiring excluded'},'summary':{'page_count':len(pages),'endpoint_token_count':len(endpoints),'naif_directory_entry_count':len(dir_entries),'api_result_payloads_ingested':False,'ephemeris_or_kernel_payloads_ingested':False,'close_approach_or_hazard_rows_ingested':False,'horizons_results_ingested':False,'mission_kernels_ingested':False,'operational_guidance_enabled':False,'runtime_execution_enabled':False,'mirror_graph_wiring':False},'pages':pages,'ssd_endpoint_tokens':endpoints[:240],'naif_directory_refs':dir_entries[:240]}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list): return '[ ]' if not v else '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict): return '{ }' if not v else '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/nasa-jpl-ssd-naif-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-space-nasa-jpl-ssd-naif-catalog.sh && scripts/gen-space-nasa-jpl-ssd-naif-catalog.sh\n'
content+='# 범위: NASA/JPL SSD/NAIF catalog metadata only. API/kernel payloads/hazard rows/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: pages={len(pages)} endpoints={len(endpoints)} naif_refs={len(dir_entries)} bytes={len(content.encode())}')
PY
