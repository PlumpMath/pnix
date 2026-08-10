#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KR_PUBLIC_BIOHEALTH_SRC:-$ROOT/ingest/korea/kr-public-biohealth-catalogs}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/kr-public-biohealth-catalogs.generated.px}"
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, html, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
def clean(s): return re.sub(r'\s+',' ',html.unescape(re.sub('<.*?>',' ',str(s or '')))).strip()
def title_of(raw):
    m=re.search(r'<title[^>]*>(.*?)</title>',raw,re.I|re.S)
    return clean(m.group(1))[:220] if m else ''
manifest=json.loads((src/'source-manifest.json').read_text(encoding='utf-8')) if (src/'source-manifest.json').exists() else {'sources':[]}
sources=[]; field_catalogs=[]; domains={}
for s in manifest.get('sources',[]):
    domain=s.get('domain','biohealth'); domains[domain]=domains.get(domain,0)+1
    pages=[]
    for fr in s.get('fetch_results',[]):
        p=src/fr.get('path',''); raw=p.read_text(encoding='utf-8',errors='ignore') if p.exists() else ''
        pages.append({'url':fr.get('url',''),'http_status':fr.get('http_status',''),'curl_exit':fr.get('curl_exit',0),'path':fr.get('path',''),'bytes':fr.get('bytes',0),'sha256':fr.get('sha256',''),'title':title_of(raw)})
    sources.append({'source_id':s.get('source_id',''),'domain':domain,'label':s.get('label',''),'license':s.get('license',''),'pages':pages,'ok_pages':sum(1 for p in pages if str(p.get('http_status','')).startswith('2'))})
    field_catalogs.append({'source_id':s.get('source_id',''),'domain':domain,'label':s.get('label',''),'stored_payload_rows':False,'declared_field_refs':s.get('declared_field_refs',[]),'excluded_payload':s.get('excluded_payload','')})
files=[{'path':str(p.relative_to(src)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(src.rglob('*')) if p.is_file()]
obj={'schema':'kr.public_biohealth_catalogs.v1','source':'Korean official public biodiversity/food/health catalog metadata','license':'KR public-data catalog metadata only; biodiversity/food/health payload rows excluded','policy':'Stores official source catalog metadata and declared field-name references for Korean biodiversity, food nutrition, medicine statistics, and disease-control public-data catalog pages. Excludes species/specimen/genetic-resource rows, nutrition values, food/product/company records, medicine item records, disease occurrence rows, patient/person records, clinical/dietary/prescription guidance, and graph/mirror wiring.','summary':{'source_count':len(sources),'ok_source_count':sum(1 for s in sources if s['ok_pages']>0),'field_catalog_count':len(field_catalogs),'domain_counts':domains,'payload_rows_ingested':False,'species_records_ingested':False,'nutrition_values_ingested':False,'medicine_item_records_ingested':False,'disease_occurrence_rows_ingested':False,'patient_or_person_records_ingested':False,'clinical_or_dietary_guidance_ingested':False,'mirror_graph_wiring':False},'field_catalogs':field_catalogs,'sources':sources,'files':files[:120],'manifest':{'retrieved_at_utc':manifest.get('retrieved_at_utc',''),'policy':manifest.get('policy','')}}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,float): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v),ensure_ascii=False)
content='# stdlib/lib/corpus/kr-public-biohealth-catalogs.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-kr-public-biohealth-catalogs.sh && scripts/gen-kr-public-biohealth-catalogs.sh\n'
content+='# 범위: 한국 공식 생물/식품/보건 catalog metadata only. records/values/guidance/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: sources={len(sources)} fields={len(field_catalogs)} bytes={len(content.encode())}')
PY
