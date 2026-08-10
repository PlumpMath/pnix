#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_BIOHEALTH_OUT:-$ROOT/ingest/korea/kr-public-biohealth-catalogs}"
UA="pnix-ingest/1.0 (Korean public biohealth catalog metadata only; no payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kr_nibr_biodiversity_metadata_info","domain":"biodiversity","label":"국립생물자원관 메타데이터정보 catalog","license":"Data.go.kr public catalog metadata; biodiversity payload excluded","urls":["https://www.data.go.kr/data/15089634/fileData.do"],"declared_field_refs":["national species list marker","KTSN marker","scientific name field marker","synonym/specimen/material/taxonomy metadata markers"],"excluded_payload":"species/specimen/material/genetic-resource rows"},
  {"source_id":"kr_nibr_national_species_list_catalog","domain":"biodiversity","label":"국립생물자원관 국가생물종목록 catalog","license":"Data.go.kr public catalog metadata; species rows excluded","urls":["https://www.data.go.kr/data/15048041/fileData.do"],"declared_field_refs":["taxon group field","Korean name field","scientific name field","author/year field","species detail URL field"],"excluded_payload":"61k species rows, taxonomic assertions as truth, genetic-resource data"},
  {"source_id":"kr_mfds_medicine_related_stats_catalog","domain":"medicine_statistics","label":"식품의약품안전처 의약품 관련 정보 catalog","license":"Data.go.kr public catalog metadata; medicine statistics payload excluded","urls":["https://www.data.go.kr/data/15020627/fileData.do"],"declared_field_refs":["manufacturer/seller status table marker","production/import/export statistics marker","approval/notification status marker"],"excluded_payload":"medicine item records, manufacturer rows, safety/usage guidance"},
  {"source_id":"kr_mfds_food_nutrition_standard_all_catalog","domain":"food_nutrition","label":"전국통합식품영양성분정보표준데이터 catalog","license":"Data.go.kr public standard-data catalog metadata; nutrition rows excluded","urls":["https://www.data.go.kr/data/15100064/standard.do"],"declared_field_refs":["food code field","food name field","nutrient basis amount field","energy/protein/fat/mineral/vitamin field names","source code field"],"excluded_payload":"food rows, nutrition values, company/product rows, dietary guidance"},
  {"source_id":"kr_mfds_food_nutrition_processed_catalog","domain":"food_nutrition","label":"전국통합식품영양성분정보 가공식품 표준데이터 catalog","license":"Data.go.kr public standard-data catalog metadata; nutrition rows excluded","urls":["https://www.data.go.kr/data/15100066/standard.do"],"declared_field_refs":["processed food classification field","representative food code/name fields","nutrition component field names","origin/manufacturer/importer field names"],"excluded_payload":"processed-food rows, nutrient values, product/company records"},
  {"source_id":"kr_mfds_food_nutrition_db_api_catalog","domain":"food_nutrition","label":"식품의약품안전처 식품영양성분DB정보 catalog","license":"Data.go.kr public catalog metadata; API response payload excluded","urls":["https://www.data.go.kr/data/15127578/openapi.do"],"declared_field_refs":["food classification field","food code field","nutrition content basis field","data creation method/date field"],"excluded_payload":"API responses, nutrition values, food product rows"},
  {"source_id":"kr_kdca_public_data_page_catalog","domain":"health_public_data","label":"질병관리청 공공데이터 page catalog","license":"KDCA public page catalog metadata only; health payload excluded","urls":["https://www.kdca.go.kr/kdca/2784/subview.do"],"declared_field_refs":["public-data page section marker","infectious-disease data collection marker","health/disease data catalog marker"],"excluded_payload":"page prose body, disease data rows, public-health advice"},
  {"source_id":"kr_kdca_notifiable_infectious_disease_api_catalog","domain":"health_statistics","label":"질병관리청 전수신고 감염병 발생현황 catalog","license":"Data.go.kr public catalog metadata; disease occurrence payload excluded","urls":["https://www.data.go.kr/data/15139178/openapi.do"],"declared_field_refs":["period disease occurrence function marker","patient class function marker","infection area function marker","region/age/sex/disease/death function markers"],"excluded_payload":"disease occurrence rows, patient/person records, response or medical guidance"}
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-biohealth-catalogs','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean biodiversity/food/health catalog metadata only; no species records, no nutrition values, no medicine records, no disease occurrence rows, no patient/person records, no guidance, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
