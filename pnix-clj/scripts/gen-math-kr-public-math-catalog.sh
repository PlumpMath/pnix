#!/usr/bin/env bash
# Korean public math catalog snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KR_PUBLIC_MATH_SRC:-$ROOT/ingest/math/kr-public-math-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/kr-public-math-catalog.generated.px}"
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
        if any(k in low for k in ['math','수학','교육과정','교수','학습','콘텐츠','openapi','filedata','api','data','json','xml','csv','pdf','보고서','공모전','놀이','kci','학술지']):
            links.append({'href':href,'label':label})
            if len(links)>=80: break
    return links
manifest=json.loads((src/'source-manifest.json').read_text(encoding='utf-8')) if (src/'source-manifest.json').exists() else {'sources':[]}
# Declared public catalog field refs only. No actual records/problems/prose are copied.
field_catalogs=[
  {'source_id':'data_go_kr_kofac_math_curriculum_api','dataset':'수학교육과정 교수학습자료 OpenAPI','stored_payload_rows':False,'field_refs':['한 페이지 결과 수','페이지 번호','전체 결과 수','현재 결과 수','정렬','카테고리','게시물 번호','제목','작성자','조회수','주관연구기관','썸네일 경로'],'excluded_payload':'API rows, teacher-guide/PDF content, classroom materials, problem/solution text'},
  {'source_id':'data_go_kr_kofac_math_curriculum_file','dataset':'수학교육과정 교수학습자료 file catalog','stored_payload_rows':False,'field_refs':['학교급','학년','학기','단원 catalog metadata','자료 URL/hash metadata candidate'],'excluded_payload':'PDF bodies, teacher-guide text, assessment material, textbook-derived content'},
  {'source_id':'data_go_kr_kofac_askmath_play_catalog','dataset':'AskMath 수학놀이 콘텐츠 catalog','stored_payload_rows':False,'field_refs':['제목','이용방법 링크','조회수','등록일자'],'excluded_payload':'YouTube/video body, play instructions, activity prose, images'},
  {'source_id':'data_go_kr_kofac_askmath_contest_catalog','dataset':'AskMath 공모전 자료 catalog','stored_payload_rows':False,'field_refs':['제목','내용 field name only','조회수','작성자','등록일자'],'excluded_payload':'contest submission body, student/teacher works, images, event reports'},
  {'source_id':'data_go_kr_kofac_askmath_report_catalog','dataset':'AskMath 연구보고서 catalog','stored_payload_rows':False,'field_refs':['보고서 catalog URL','제목 field name','작성/기관 metadata candidate'],'excluded_payload':'research report PDF/body/prose, evaluation tools, curriculum-development text'},
  {'source_id':'data_go_kr_nsm_science_learning_catalog','dataset':'국립중앙과학관 과학학습콘텐츠 catalog','stored_payload_rows':False,'field_refs':['학습 주제 id','기관/참고자료 link metadata','수학 용어/수학 공식 topic marker'],'excluded_payload':'Wikipedia/Doosan/third-party encyclopedia content, formulas prose, learning-content bodies'},
  {'source_id':'data_go_kr_nrf_kci_journal_catalog','dataset':'KCI학술지정보 catalog','stored_payload_rows':False,'field_refs':['ISSN','학술지명','등재 구분','연구 분야','창간년도','발행 간기','사용 언어','발행기관'],'excluded_payload':'article records, abstracts, citations, full texts, author/person payload'}
]
math_topic_markers=[
  {'marker':'수학','kind':'korean_math_domain_token','source_refs':['data_go_kr_kofac_math_curriculum_api','data_go_kr_kofac_math_curriculum_file']},
  {'marker':'수학교육','kind':'korean_math_education_token','source_refs':['data_go_kr_kofac_math_curriculum_api','data_go_kr_kofac_askmath_report_catalog']},
  {'marker':'수학 용어','kind':'korean_math_term_catalog_marker','source_refs':['data_go_kr_nsm_science_learning_catalog']},
  {'marker':'수학 공식','kind':'korean_math_formula_catalog_marker','source_refs':['data_go_kr_nsm_science_learning_catalog']},
  {'marker':'KCI 수학 학술지','kind':'korean_math_research_catalog_marker','source_refs':['data_go_kr_nrf_kci_journal_catalog']}
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
obj={'schema':'math.kr_public_math_catalog.v1','source':'Korean official public math-related catalog metadata','license':'KR public-data catalog metadata only; payload/prose/problem/PDF/video/article bodies excluded','policy':'Stores source catalog metadata and declared field names for Korean math education/science-learning/research catalog sources. Excludes instructional prose, PDFs, videos, math problems, solutions, contest submissions, third-party encyclopedia content, article payloads, API keys, and graph/mirror wiring.','summary':{'source_count':len(sources),'ok_source_count':sum(1 for s in sources if s['ok_pages']>0),'field_catalog_count':len(field_catalogs),'math_topic_marker_count':len(math_topic_markers),'payload_rows_ingested':False,'prose_bodies_ingested':False,'pdf_bodies_ingested':False,'problem_solution_ingested':False,'video_payloads_ingested':False,'third_party_encyclopedia_content_ingested':False,'mirror_graph_wiring':False},'field_catalogs':field_catalogs,'math_topic_markers':math_topic_markers,'sources':sources,'files':files[:90],'manifest':{'retrieved_at_utc':manifest.get('retrieved_at_utc',''),'policy':manifest.get('policy','')}}
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
content='# stdlib/lib/corpus/kr-public-math-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-math-kr-public-math-catalog.sh && scripts/gen-math-kr-public-math-catalog.sh\n'
content+='# 범위: 한국 공식 수학 관련 catalog metadata only. PDF/prose/problem/solution/video/article payload/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: sources={len(sources)} fields={len(field_catalogs)} bytes={len(content.encode())}')
PY
