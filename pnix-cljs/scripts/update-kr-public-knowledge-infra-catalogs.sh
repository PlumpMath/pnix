#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_KNOWLEDGE_INFRA_OUT:-$ROOT/ingest/korea/kr-public-knowledge-infra-catalogs}"
UA="pnix-ingest/1.0 (Korean public knowledge infra catalog metadata only; no payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kr_nics_chemical_safety_management_api","domain":"chemistry_safety","label":"화학물질안전원 화학물질 안전관리정보 catalog","license":"Data.go.kr public catalog metadata; chemical payload excluded","urls":["https://www.data.go.kr/data/15072442/openapi.do"],"declared_field_refs":["Korean substance name field","English substance name field","CAS number field","symptom/exposure field names"],"excluded_payload":"chemical rows, symptom text, handling or safety decisions"},
  {"source_id":"kr_keco_toxic_ghs_api","domain":"chemistry_safety","label":"한국환경공단 유독물GHS 정보 조회 서비스 catalog","license":"Data.go.kr public catalog metadata; GHS payload excluded","urls":["https://www.data.go.kr/data/15149423/openapi.do"],"declared_field_refs":["CAS number field","toxic substance id field","signal word field","UN number field","hazard-class field names","pictogram code field"],"excluded_payload":"GHS hazard detail rows, chemical handling guidance"},
  {"source_id":"kr_nics_chemical_accident_api","domain":"chemistry_safety","label":"화학물질안전원 화학사고정보 catalog","license":"Data.go.kr public catalog metadata; accident payload excluded","urls":["https://www.data.go.kr/data/15072446/openapi.do"],"declared_field_refs":["accident date field","cause field","material type field","damage scale field names"],"excluded_payload":"accident rows, response/evacuation guidance, facility locations"},
  {"source_id":"kr_nics_chemical_release_transfer_api","domain":"chemistry_environment","label":"화학물질 배출 및 이동량 정보 catalog","license":"Data.go.kr public catalog metadata; release/transfer payload excluded","urls":["https://www.data.go.kr/data/15024756/openapi.do"],"declared_field_refs":["release amount field name","transfer amount field name","chemical id field","business site field marker"],"excluded_payload":"facility rows, release values, compliance or risk judgment"},
  {"source_id":"kr_kipris_patent_utility_api","domain":"intellectual_property","label":"KIPRISPlus 특허·실용 공개/등록공보 REST API catalog","license":"Data.go.kr public catalog metadata; patent payload excluded","urls":["https://www.data.go.kr/data/15065437/openapi.do?recommendDataYn=Y"],"declared_field_refs":["application number field","registration date field","invention title field","IPC code field","publication/registration number field"],"excluded_payload":"patent full text, claims, drawings, abstracts, applicant/inventor payload rows"},
  {"source_id":"kr_kipris_foreign_patent_api","domain":"intellectual_property","label":"KIPRISPlus 해외특허 REST API catalog","license":"Data.go.kr public catalog metadata; foreign patent payload excluded","urls":["https://www.data.go.kr/data/15058701/openapi.do"],"declared_field_refs":["foreign bibliographic info field","abstract field marker","claims field marker","drawing/full-text download marker"],"excluded_payload":"foreign patent documents, abstracts, claims, drawings"},
  {"source_id":"kr_kipris_super_citation_catalog","domain":"intellectual_property","label":"KIPRIS 슈퍼인용 catalog","license":"Data.go.kr public catalog metadata; citation payload excluded","urls":["https://www.data.go.kr/data/15089856/fileData.do"],"declared_field_refs":["citation relation marker","patent information network marker","XML/JSON converted API marker"],"excluded_payload":"citation graph rows, patent payload, IP legal evaluation"},
  {"source_id":"kr_kipris_search_service_catalog","domain":"intellectual_property","label":"KIPRIS 지식재산정보 검색서비스 catalog","license":"Data.go.kr public catalog metadata; PDF/search payload excluded","urls":["https://www.data.go.kr/data/15066205/fileData.do"],"declared_field_refs":["patent/trademark/design/trial category marker","search result URL marker","PDF media type marker"],"excluded_payload":"search result rows, PDF bodies, IP rights/legal advice"},
  {"source_id":"kr_nlk_holdings_openapi_catalog","domain":"library_bibliography","label":"국립중앙도서관 소장자료 OPEN API catalog","license":"Data.go.kr public catalog metadata; bibliographic item payload excluded","urls":["https://www.data.go.kr/data/3078981/openapi.do?recommendDataYn=Y"],"declared_field_refs":["title query field","author field","publisher field","call number field","copyright info field marker"],"excluded_payload":"bibliographic item records, book metadata rows, full text"},
  {"source_id":"kr_nlk_isbn_bibliography_api","domain":"library_bibliography","label":"국립중앙도서관 ISBN서지정보 catalog","license":"Data.go.kr public catalog metadata; ISBN item payload excluded","urls":["https://www.data.go.kr/data/3078982/openapi.do"],"declared_field_refs":["ISBN field","title field","author field","publisher field","publication date field","keyword field"],"excluded_payload":"ISBN bibliographic rows, publisher-provided item records"},
  {"source_id":"kr_nlk_bibliographic_service_api","domain":"library_bibliography","label":"국립중앙도서관 서지 정보 제공 서비스 catalog","license":"Data.go.kr public catalog metadata; MARC/MODS payload excluded","urls":["https://www.data.go.kr/data/15154402/openapi.do"],"declared_field_refs":["MARC marker","MODS marker","Dublin Core marker","BIBO marker","LOD metadata marker"],"excluded_payload":"MARC/MODS item records, full metadata corpus, text bodies"},
  {"source_id":"kr_vworld_2d_map_api","domain":"spatial_api","label":"국토교통부 브이월드 2D 지도 API catalog","license":"Data.go.kr public catalog metadata; map payload excluded","urls":["https://www.data.go.kr/data/3052419/openapi.do?recommendDataYn=Y"],"declared_field_refs":["WMS marker","authentication key marker","2D map API marker","national spatial information marker"],"excluded_payload":"map tiles, geometries, coordinates, API key"},
  {"source_id":"kr_vworld_2d_mobile_api","domain":"spatial_api","label":"국토교통부 2D 모바일 API catalog","license":"Data.go.kr public catalog metadata; mobile map payload excluded","urls":["https://www.data.go.kr/data/15140369/openapi.do?recommendDataYn=Y"],"declared_field_refs":["mobile map API marker","iOS/Android support marker","WMS marker","spatial platform marker"],"excluded_payload":"map payload, geometry editing payload, API key"},
  {"source_id":"kr_vworld_static_map_api","domain":"spatial_api","label":"국토교통부 Static Map API catalog","license":"Data.go.kr public catalog metadata; static map image payload excluded","urls":["https://www.data.go.kr/data/15101108/openapi.do?recommendDataYn=Y"],"declared_field_refs":["static map API marker","background map field marker","image response marker","location display field marker"],"excluded_payload":"static map images, coordinates as payload, API key"}
]
JSON
python3 - "$OUT" "$UA" <<'PY'
import datetime, hashlib, json, pathlib, subprocess, sys
out=pathlib.Path(sys.argv[1]); ua=sys.argv[2]
sources=json.loads((out/'sources.json').read_text(encoding='utf-8'))
for src in sources:
    d=out/'pages'/src['source_id']; d.mkdir(parents=True, exist_ok=True); results=[]
    for i,u in enumerate(src['urls']):
        path=d/f'{i:02d}.html'; cmd=['curl','-L','--max-time','35','--retry','1','-A',ua,'-sS','-w','\nPNIX_HTTP_STATUS:%{http_code}\n',u]
        try:
            cp=subprocess.run(cmd,text=True,capture_output=True,timeout=45); txt=cp.stdout; status='000'; marker='PNIX_HTTP_STATUS:'
            if marker in txt:
                body,tail=txt.rsplit(marker,1); status=tail.strip().split()[0]; txt=body
            path.write_text(txt,encoding='utf-8',errors='ignore')
            results.append({'url':u,'path':str(path.relative_to(out)),'http_status':status,'curl_exit':cp.returncode,'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
        except Exception as e:
            fail=d/f'{i:02d}.error.txt'; fail.write_text(str(e),encoding='utf-8')
            results.append({'url':u,'path':str(fail.relative_to(out)),'http_status':'000','curl_exit':999,'error':str(e),'bytes':fail.stat().st_size,'sha256':hashlib.sha256(fail.read_bytes()).hexdigest()})
    src['fetch_results']=results
files=[{'path':str(p.relative_to(out)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.rglob('*')) if p.is_file() and p.name!='source-manifest.json']
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-knowledge-infra-catalogs','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean chemistry/IP/library/spatial catalog metadata only; no payload rows, no patent/library/map/chemical bodies, no credentials, no guidance, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
