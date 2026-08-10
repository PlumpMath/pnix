#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_AUTHORITY_OUT:-$ROOT/ingest/korea/kr-public-authority-catalogs}"
UA="pnix-ingest/1.0 (Korean public authority catalog metadata only; no payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kr_molit_legal_dong_code_file","domain":"admin_code","label":"국토교통부 법정동코드 catalog","license":"Data.go.kr public catalog metadata; code rows excluded","urls":["https://www.data.go.kr/data/15123287/fileData.do"],"declared_field_refs":["legal-dong code field","PNU/region code field","name field","existence flag field"],"excluded_payload":"full code rows, address/location decisions"},
  {"source_id":"kr_mois_standard_legal_dong_code_api","domain":"admin_code","label":"행정안전부 행정표준코드 법정동코드 API catalog","license":"Data.go.kr public catalog metadata; API responses excluded","urls":["https://www.data.go.kr/data/15077871/openapi.do"],"declared_field_refs":["standard code query parameter","legal-dong code response field","code management system reference"],"excluded_payload":"API response rows, code-table dump"},
  {"source_id":"kr_mois_all_standard_codes_download","domain":"admin_code","label":"행정안전부 행정표준코드 전체코드 다운로드 catalog","license":"Data.go.kr public catalog metadata; code payload excluded","urls":["https://www.data.go.kr/data/15092039/fileData.do"],"declared_field_refs":["standard-code system identifier","code category field","code value field","code name field"],"excluded_payload":"full administrative code payload"},
  {"source_id":"kr_mois_realtime_address_search_api","domain":"address","label":"행정안전부 실시간 주소정보 조회 검색API catalog","license":"Data.go.kr public catalog metadata; address query/response payload excluded","urls":["https://www.data.go.kr/data/15057017/openapi.do"],"declared_field_refs":["approval key parameter","page number parameter","address keyword parameter","result format parameter","road/address result field names"],"excluded_payload":"address result rows, API keys, personal/location payload"},
  {"source_id":"kr_mois_road_address_daily_db_catalog","domain":"address","label":"행정안전부 도로명주소 주소DB 일간 catalog","license":"Data.go.kr public catalog metadata; address DB payload excluded","urls":["https://www.data.go.kr/data/15050417/fileData.do"],"declared_field_refs":["road name address DB file reference","daily update field","TXT media type field"],"excluded_payload":"road-address DB rows, building/unit address payload"},
  {"source_id":"kr_pps_nara_bid_public_info_api","domain":"procurement","label":"조달청 나라장터 입찰공고정보서비스 catalog","license":"Data.go.kr public catalog metadata; bid payload excluded","urls":["https://www.data.go.kr/data/15129394/openapi.do"],"declared_field_refs":["bid notice list operation","bid detail operation","base amount operation","license restriction operation","eligible area operation"],"excluded_payload":"bid notices, amounts, bidders, contract decisions"},
  {"source_id":"kr_pps_nara_open_standard_service","domain":"procurement","label":"조달청 나라장터 공공데이터개방표준서비스 catalog","license":"Data.go.kr public catalog metadata; bid/award/contract payload excluded","urls":["https://www.data.go.kr/data/15058815/openapi.do"],"declared_field_refs":["bid opening standard operation","award standard operation","contract date query field","public-data open standard reference"],"excluded_payload":"bid/award/contract rows, business records"},
  {"source_id":"kr_pps_nara_order_plan_file","domain":"procurement","label":"조달청 나라장터 발주계획 내역 catalog","license":"Data.go.kr public catalog metadata; semi-structured report payload excluded","urls":["https://www.data.go.kr/data/15053351/fileData.do"],"declared_field_refs":["order plan report name","order amount field name","semi-structured report metadata"],"excluded_payload":"order plan rows, amounts, procurement advice"},
  {"source_id":"kr_moleg_national_law_info_api","domain":"law","label":"법제처 국가법령정보 공유서비스 catalog","license":"Data.go.kr public catalog metadata; law text payload excluded","urls":["https://www.data.go.kr/data/15000115/openapi.do"],"declared_field_refs":["law id field","article number field","effective date field","revision history field","competent ministry field"],"excluded_payload":"law article text, annexes, legal advice"},
  {"source_id":"kr_moleg_life_law_info_api","domain":"law","label":"법제처 생활법령정보 조회 서비스 catalog","license":"KOGL-type catalog metadata; life-law payload excluded","urls":["https://www.data.go.kr/data/15000215/openapi.do"],"declared_field_refs":["SOAP API type","XML format","life-law lookup keyword","traffic/account metadata"],"excluded_payload":"life-law explanation bodies, legal advice"},
  {"source_id":"kr_moleg_legal_interpretation_detail_api","domain":"law","label":"법제처 법령해석례 상세 조회 정보 catalog","license":"Data.go.kr public catalog metadata; interpretation payload excluded","urls":["https://www.data.go.kr/data/15090746/openapi.do"],"declared_field_refs":["question receipt number field","requesting institution field","target law article field","interpretation answer field"],"excluded_payload":"interpretation request/answer bodies, precedents, legal reasoning payload"},
  {"source_id":"kr_moleg_current_law_body_api","domain":"law","label":"법제처 현행법령 본문 조회 catalog","license":"Data.go.kr public catalog metadata; law body payload excluded","urls":["https://www.data.go.kr/data/15057358/openapi.do"],"declared_field_refs":["current law lookup operation","article unit field","law body field name","effective law status field"],"excluded_payload":"current law body text, article text, legal compliance advice"}
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-authority-catalogs','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean authority lookup catalog metadata only; no code/address/law/procurement payload rows, no API responses, no credentials, no guidance, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
