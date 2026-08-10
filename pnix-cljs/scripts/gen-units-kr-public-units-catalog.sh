#!/usr/bin/env bash
# Korean public unit-related official source snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KR_PUBLIC_UNITS_SRC:-$ROOT/ingest/units/kr-public-units-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/kr-public-units-catalog.generated.px}"
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, html, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
def clean(s): return re.sub(r'\s+',' ',html.unescape(re.sub('<.*?>',' ',str(s or '')))).strip()
def title_of(raw):
    m=re.search(r'<title[^>]*>(.*?)</title>',raw,re.I|re.S)
    return clean(m.group(1))[:220] if m else ''
def links_of(raw):
    links=[]
    for m in re.finditer(r'<a\s+[^>]*href=["\']([^"\']+)["\'][^>]*>(.*?)</a>',raw,re.I|re.S):
        href=html.unescape(m.group(1)); label=clean(m.group(2))[:120]
        low=(href+' '+label).lower()
        if any(k in low for k in ['unit','단위','법정','표준','계량','api','data','csv','json','download','filedata','openapi','ks','si']):
            links.append({'href':href,'label':label})
            if len(links)>=80: break
    return links
manifest=json.loads((src/'source-manifest.json').read_text(encoding='utf-8')) if (src/'source-manifest.json').exists() else {'sources':[]}
# Facts-only Korean legal SI base-unit anchors. These are small legal/metrological facts, not statute prose.
base_units=[
  {'quantity_ko':'길이','quantity_en':'length','unit_name_ko':'미터','unit_name_en':'metre','symbol':'m','si_base_index':1,'qudt_hint':'M','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
  {'quantity_ko':'질량','quantity_en':'mass','unit_name_ko':'킬로그램','unit_name_en':'kilogram','symbol':'kg','si_base_index':2,'qudt_hint':'KiloGM','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
  {'quantity_ko':'시간','quantity_en':'time','unit_name_ko':'초','unit_name_en':'second','symbol':'s','si_base_index':3,'qudt_hint':'SEC','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
  {'quantity_ko':'전류','quantity_en':'electric current','unit_name_ko':'암페어','unit_name_en':'ampere','symbol':'A','si_base_index':4,'qudt_hint':'A','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
  {'quantity_ko':'온도','quantity_en':'thermodynamic temperature','unit_name_ko':'켈빈','unit_name_en':'kelvin','symbol':'K','si_base_index':5,'qudt_hint':'K','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
  {'quantity_ko':'물질량','quantity_en':'amount of substance','unit_name_ko':'몰','unit_name_en':'mole','symbol':'mol','si_base_index':6,'qudt_hint':'MOL','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
  {'quantity_ko':'광도','quantity_en':'luminous intensity','unit_name_ko':'칸델라','unit_name_en':'candela','symbol':'cd','si_base_index':7,'qudt_hint':'CD','law_basis':['국가표준기본법 제10조','국가표준기본법 시행령 기본단위 기호 조항'],'source_refs':['kats_legal_units_overview','law_national_standards_framework_decree']},
]
legal_unit_classes=[
  {'class_ko':'기본단위','class_en':'base unit','source_refs':['kats_legal_units_overview','law_measurement_act']},
  {'class_ko':'유도단위','class_en':'derived unit','source_refs':['kats_legal_units_overview','law_measurement_act']},
  {'class_ko':'특수단위','class_en':'special unit','source_refs':['kats_legal_units_overview','law_measurement_act']},
]
unit_related_dataset_fields=[
  {'source_id':'data_go_kr_agri_trade_unit_mapping_api','dataset':'농수축산물 거래단량 매핑 정보조회','stored_payload_rows':False,'field_refs':['거래단위 코드','거래단위 명칭','표준단위 코드','표준단위 명칭','환산비율'],'excluded_payload':'API result rows, market prices, product/commodity price payloads'},
  {'source_id':'data_go_kr_agri_standard_unit_mapping_file','dataset':'농수축산물 표준코드-조사가격단위매핑목록','stored_payload_rows':False,'field_refs':['단위코드(stdUnitCode)','단위명(stdUnitNm)','조사단위명 산지','조사단위명 도매','조사단위명 소매','업데이트일자'],'excluded_payload':'file rows not downloaded in this slice; only catalog/field schema metadata'},
  {'source_id':'data_go_kr_kats_national_standard_catalog','dataset':'이나라표준인증 국가표준','stored_payload_rows':False,'field_refs':['KS identifier/catalog metadata','standard title/category/status/date','unit/term/quantity related catalog filter candidate'],'excluded_payload':'KS/ISO standard document bodies and prose'},
]
sources=[]
for s in manifest.get('sources',[]):
    pages=[]
    for fr in s.get('fetch_results',[]):
        p=src/fr.get('path','')
        raw=p.read_text(encoding='utf-8',errors='ignore') if p.exists() else ''
        pages.append({'url':fr.get('url',''),'http_status':fr.get('http_status',''),'curl_exit':fr.get('curl_exit',0),'path':fr.get('path',''),'bytes':fr.get('bytes',0),'sha256':fr.get('sha256',''),'title':title_of(raw),'selected_links':links_of(raw)})
    sources.append({'source_id':s.get('source_id',''),'label':s.get('label',''),'license':s.get('license',''),'pages':pages,'ok_pages':sum(1 for p in pages if str(p.get('http_status','')).startswith('2'))})
files=[]
for p in sorted(src.rglob('*')):
    if p.is_file(): files.append({'path':str(p.relative_to(src)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
obj={'schema':'units.kr_public_units_catalog.v1','source':'Korean official public units catalog metadata and legal-unit facts','license':'KR official public metadata / KOGL Type 1 or legal facts where applicable; source prose and payload rows excluded','policy':'Stores facts-only SI legal base-unit anchors and official source catalog metadata. Excludes statute full text, standards document bodies, webpage prose bodies, PDF/HWP bodies, API result rows, market/price payloads, compliance/legal advice, and graph/mirror wiring.','summary':{'legal_base_unit_count':len(base_units),'legal_unit_class_count':len(legal_unit_classes),'source_count':len(sources),'ok_source_count':sum(1 for s in sources if s['ok_pages']>0),'unit_related_dataset_field_catalogs':len(unit_related_dataset_fields),'payload_rows_ingested':False,'webpage_prose_bodies_ingested':False,'standards_bodies_ingested':False,'mirror_graph_wiring':False},'legal_base_units':base_units,'legal_unit_classes':legal_unit_classes,'unit_related_dataset_fields':unit_related_dataset_fields,'sources':sources,'files':files[:80],'manifest':{'retrieved_at_utc':manifest.get('retrieved_at_utc',''),'policy':manifest.get('policy','')}}
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
content='# stdlib/lib/corpus/kr-public-units-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-units-kr-public-units-catalog.sh && scripts/gen-units-kr-public-units-catalog.sh\n'
content+='# 범위: 한국 공식 단위 facts/catalog metadata only. 원문/표준본문/payload/advice/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: legal_base_units={len(base_units)} sources={len(sources)} bytes={len(content.encode())}')
PY
