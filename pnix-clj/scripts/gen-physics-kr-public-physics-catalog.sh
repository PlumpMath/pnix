#!/usr/bin/env bash
# Korean public physics catalog snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KR_PUBLIC_PHYSICS_SRC:-$ROOT/ingest/physics/kr-public-physics-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/kr-public-physics-catalog.generated.px}"
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, html, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
def clean(s): return re.sub(r'\s+',' ',html.unescape(re.sub('<.*?>',' ',str(s or '')))).strip()
def title_of(raw):
    m=re.search(r'<title[^>]*>(.*?)</title>',raw,re.I|re.S)
    return clean(m.group(1))[:220] if m else ''
def selected_links(raw):
    links=[]
    for m in re.finditer(r'<a\s+[^>]*href=["\']([^"\']+)["\'][^>]*>(.*?)</a>',raw,re.I|re.S):
        href=html.unescape(m.group(1)); label=clean(m.group(2))[:120]
        low=(href+' '+label).lower()
        if any(k in low for k in ['physics','물리','과학','천문','분광','우주','원자력','방사선','강의','학술지','openapi','filedata','api','data','json','xml','csv','콘텐츠','과학기술']):
            links.append({'href':href,'label':label})
            if len(links)>=80: break
    return links
manifest=json.loads((src/'source-manifest.json').read_text(encoding='utf-8')) if (src/'source-manifest.json').exists() else {'sources':[]}
field_catalogs=[
  {'source_id':'data_go_kr_kofac_smart_science_math_lab','dataset':'스마트 수과학실 catalog','stored_payload_rows':False,'field_refs':['기관/시설명','지역/주소 field name','프로그램/운영 정보 field name','연결 URL field name'],'excluded_payload':'program content, precise student participation records, classroom materials'},
  {'source_id':'data_go_kr_nsm_science_learning_catalog','dataset':'과학학습콘텐츠 catalog','stored_payload_rows':False,'field_refs':['학습 주제 id','기관/참고자료 link metadata','물리/천문/과학 topic marker'],'excluded_payload':'third-party encyclopedia content, learning prose, formulas/experiment explanations'},
  {'source_id':'data_go_kr_kofac_excellent_science_books','dataset':'우수과학도서 정보 catalog','stored_payload_rows':False,'field_refs':['도서명 field name','분야/연도 field name','저자/출판사 field name','추천/선정 metadata field name'],'excluded_payload':'book content, reviews/prose, cover/media payload'},
  {'source_id':'data_go_kr_nsm_science_fair_awards','dataset':'국립중앙과학관 수상작정보 catalog','stored_payload_rows':False,'field_refs':['대회명','행사년도','주제','소속명 field name','제목 field name','수상명'],'excluded_payload':'award work bodies, reports, images, personal/student payload rows'},
  {'source_id':'data_go_kr_youth_space_center_spectrum','dataset':'천문관측 분광 catalog','stored_payload_rows':False,'field_refs':['분광형','자료 유형','촬영/관측 source metadata','파일 format metadata'],'excluded_payload':'JPEG/RAW spectrum images, analysis values, observation payload files'},
  {'source_id':'data_go_kr_kocw_course_catalog','dataset':'KOCW 공개강의서비스정보 catalog','stored_payload_rows':False,'field_refs':['교육 대분류명','교육 중분류명','공개강의명 field name','공개강의설명 field name','제공대학명','교수명','강의URL주소'],'excluded_payload':'course descriptions/prose, lecture videos, slide/PDF bodies, quizzes/problems'},
  {'source_id':'data_go_kr_nrf_kci_journal_catalog','dataset':'KCI학술지정보 catalog','stored_payload_rows':False,'field_refs':['ISSN','학술지명','등재 구분','연구 분야','창간년도','발행 간기','사용 언어','발행기관'],'excluded_payload':'article records, abstracts, citations, full texts, author/person payload'},
  {'source_id':'data_go_kr_kirams_paper_catalog','dataset':'한국원자력의학원 논문정보 catalog','stored_payload_rows':False,'field_refs':['논문번호 field name','학술지구분','논문제목 field name','학술지명칭','게재일자'],'excluded_payload':'paper records, titles/authors as payload rows, abstracts/full texts, medical/radiation advice'},
  {'source_id':'data_go_kr_kofac_sciencetimes_scitech','dataset':'사이언스타임즈 과학기술 catalog','stored_payload_rows':False,'field_refs':['category URL','article-list catalog metadata field names'],'excluded_payload':'article body, images, comments, external media'}
]
physics_topic_markers=[
  {'marker':'물리','kind':'korean_physics_domain_token','source_refs':['data_go_kr_nsm_science_learning_catalog','data_go_kr_kocw_course_catalog']},
  {'marker':'천문','kind':'korean_astrophysics_catalog_token','source_refs':['data_go_kr_youth_space_center_spectrum','data_go_kr_nsm_science_learning_catalog']},
  {'marker':'분광','kind':'korean_spectroscopy_catalog_token','source_refs':['data_go_kr_youth_space_center_spectrum']},
  {'marker':'원자력','kind':'korean_nuclear_physics_catalog_token','source_refs':['data_go_kr_kirams_paper_catalog']},
  {'marker':'방사선','kind':'korean_radiation_physics_catalog_token','source_refs':['data_go_kr_kirams_paper_catalog']},
  {'marker':'과학기술','kind':'korean_science_technology_catalog_token','source_refs':['data_go_kr_kofac_sciencetimes_scitech','data_go_kr_kofac_excellent_science_books']}
]
sources=[]
for s in manifest.get('sources',[]):
    pages=[]
    for fr in s.get('fetch_results',[]):
        p=src/fr.get('path','')
        raw=p.read_text(encoding='utf-8',errors='ignore') if p.exists() else ''
        pages.append({'url':fr.get('url',''),'http_status':fr.get('http_status',''),'curl_exit':fr.get('curl_exit',0),'path':fr.get('path',''),'bytes':fr.get('bytes',0),'sha256':fr.get('sha256',''),'title':title_of(raw),'selected_links':selected_links(raw)})
    sources.append({'source_id':s.get('source_id',''),'label':s.get('label',''),'license':s.get('license',''),'pages':pages,'ok_pages':sum(1 for p in pages if str(p.get('http_status','')).startswith('2'))})
files=[]
for p in sorted(src.rglob('*')):
    if p.is_file(): files.append({'path':str(p.relative_to(src)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
obj={'schema':'physics.kr_public_physics_catalog.v1','source':'Korean official public physics-related catalog metadata','license':'KR public-data catalog metadata only; payload/prose/problem/media/article bodies excluded','policy':'Stores source catalog metadata and declared field names for Korean physics/STEM/science-learning/course/research catalog sources. Excludes instructional prose, images, spectrum files, lecture media, experiment procedures, physics problems, solutions, article/paper payloads, operational nuclear/radiation advice, API keys, and graph/mirror wiring.','summary':{'source_count':len(sources),'ok_source_count':sum(1 for s in sources if s['ok_pages']>0),'field_catalog_count':len(field_catalogs),'physics_topic_marker_count':len(physics_topic_markers),'payload_rows_ingested':False,'prose_bodies_ingested':False,'image_or_spectrum_payloads_ingested':False,'problem_solution_ingested':False,'course_media_ingested':False,'article_or_paper_payloads_ingested':False,'operational_nuclear_or_radiation_advice_ingested':False,'mirror_graph_wiring':False},'field_catalogs':field_catalogs,'physics_topic_markers':physics_topic_markers,'sources':sources,'files':files[:100],'manifest':{'retrieved_at_utc':manifest.get('retrieved_at_utc',''),'policy':manifest.get('policy','')}}
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
content='# stdlib/lib/corpus/kr-public-physics-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-physics-kr-public-physics-catalog.sh && scripts/gen-physics-kr-public-physics-catalog.sh\n'
content+='# 범위: 한국 공식 물리 관련 catalog metadata only. prose/image/problem/solution/media/article payload/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: sources={len(sources)} fields={len(field_catalogs)} bytes={len(content.encode())}')
PY
