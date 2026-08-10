#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_ECONOMY_OUT:-$ROOT/ingest/korea/kr-public-economy-catalogs}"
UA="pnix-ingest/1.0 (Korean public economy catalog metadata only; no payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kr_moef_integrated_fiscal_core_catalog","domain":"fiscal","label":"기획재정부 통합재정정보 국가중점데이터 catalog","license":"Data.go.kr public catalog metadata; fiscal payload excluded","urls":["https://www.data.go.kr/tcs/eds/selectCoreDataView.do?coreDataInsttCode=1052000&coreDataSn=1"],"declared_field_refs":["open fiscal data system reference","budget/expenditure catalog class","fiscal support information catalog marker"],"excluded_payload":"budget/expenditure rows, fiscal analysis, policy advice"},
  {"source_id":"kr_moef_annual_expenditure_budget_api","domain":"fiscal","label":"기획재정부 연도별 세출 및 지출 예산현황 catalog","license":"Data.go.kr public catalog metadata; budget values excluded","urls":["https://www.data.go.kr/data/3073756/openapi.do"],"declared_field_refs":["year field","expenditure budget field name","fund operation plan field name"],"excluded_payload":"budget amount rows, fiscal adequacy judgments"},
  {"source_id":"kr_moef_state_property_api","domain":"fiscal_asset","label":"기획재정부 국유재산 증감 및 현재액 현황 catalog","license":"Data.go.kr public catalog metadata; asset values excluded","urls":["https://www.data.go.kr/data/15054319/openapi.do"],"declared_field_refs":["central agency field","account field","state property change field","current amount field name"],"excluded_payload":"asset value rows, legal/fiscal advice"},
  {"source_id":"kr_moef_public_institution_info_api","domain":"public_institution","label":"공공기관 정보 조회 서비스 catalog","license":"Data.go.kr public catalog metadata; institution payload excluded","urls":["https://www.data.go.kr/data/15125287/openapi.do"],"declared_field_refs":["institution name field","institution type field","responsible ministry field","branch/location field names"],"excluded_payload":"institution/branch records, contact/person payload"},
  {"source_id":"kr_bok_national_accounts_api","domain":"economics","label":"한국은행 국민계정 catalog","license":"Data.go.kr public catalog metadata; statistics values excluded","urls":["https://www.data.go.kr/data/15059629/openapi.do"],"declared_field_refs":["national accounts statistical series marker","growth rate field name","income/statistics query metadata"],"excluded_payload":"national accounts values, economic interpretation"},
  {"source_id":"kr_bok_monetary_financial_statistics_api","domain":"finance_statistics","label":"한국은행 통화금융통계 catalog","license":"Data.go.kr public catalog metadata; time-series values excluded","urls":["https://www.data.go.kr/data/15059638/openapi.do"],"declared_field_refs":["ECOS API reference","monetary base marker","broad money marker","JSON/XML format metadata"],"excluded_payload":"monetary/financial time series, trading signals"},
  {"source_id":"kr_bok_payment_settlement_statistics_api","domain":"payment_statistics","label":"한국은행 지급결제통계 catalog","license":"Data.go.kr public catalog metadata; payment values excluded","urls":["https://www.data.go.kr/data/15059632/openapi.do"],"declared_field_refs":["payment system statistics marker","payment method field name","institution participant field name"],"excluded_payload":"payment statistics values, financial-risk advice"},
  {"source_id":"kr_bok_business_survey_api","domain":"business_statistics","label":"한국은행 기업경기조사 catalog","license":"Data.go.kr public catalog metadata; survey values excluded","urls":["https://www.data.go.kr/data/15059630/openapi.do"],"declared_field_refs":["business survey marker","sales/business condition field name","new orders field name"],"excluded_payload":"survey values, firm records, forecasts"},
  {"source_id":"kr_bok_producer_price_api","domain":"price_statistics","label":"한국은행 생산자물가조사 catalog","license":"Data.go.kr public catalog metadata; price values excluded","urls":["https://www.data.go.kr/data/15059642/openapi.do"],"declared_field_refs":["producer price index marker","item/service price field name","statistics API metadata"],"excluded_payload":"price index values, inflation/trading advice"},
  {"source_id":"kr_kosis_shared_service_api","domain":"statistics_catalog","label":"국가데이터처 KOSIS 공유서비스 catalog","license":"Data.go.kr public catalog metadata; statistical payload excluded","urls":["https://www.data.go.kr/data/15059039/openapi.do"],"declared_field_refs":["KOSIS shared service marker","domestic/international/North Korea statistics catalog marker","XML/JSON format metadata"],"excluded_payload":"statistical values, publication content"},
  {"source_id":"kr_kosis_survey_item_info_catalog","domain":"statistics_metadata","label":"국가데이터처 KOSIS 조사항목정보 catalog","license":"Data.go.kr public catalog metadata; item payload excluded","urls":["https://www.data.go.kr/data/15136538/fileData.do"],"declared_field_refs":["survey item id field","item name field","required flag field","publication category field","item order field"],"excluded_payload":"survey item rows, respondent data"},
  {"source_id":"kr_kosis_survey_item_description_catalog","domain":"statistics_metadata","label":"국가데이터처 KOSIS 통계조사 항목설명 catalog","license":"Data.go.kr public catalog metadata; description payload excluded","urls":["https://www.data.go.kr/data/15136597/fileData.do"],"declared_field_refs":["survey item explanation file metadata","RestAPI conversion metadata","JSON/XML format metadata"],"excluded_payload":"item description rows, prose bodies"}
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-economy-catalogs','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean fiscal/economy/statistics catalog metadata only; no time-series values, no budget/tax/business/person payload rows, no forecasts, no advice, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
