#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/catalog/remaining-allowlist-public-catalogs"
mkdir -p "$OUT/pages"
UA="pnix-ingest/1.0 (remaining allow-list catalog metadata only; no payload records)"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kostat_classification_portal","label":"통계청 통계분류포털 표준분류","license":"KR official public catalog metadata","urls":["https://kssc.kostat.go.kr/ksscNew_web/kssc/main/main.do?gubun=1","https://kssc.kostat.go.kr:8443/ksscNew_web/index.jsp"]},
  {"source_id":"kisti_snt_terms","label":"KISTI 과학기술표준용어","license":"KISTI/DataON public catalog metadata; payload excluded","urls":["https://aida.kisti.re.kr/data/1da6cd4c-45e1-41ea-a457-a92d7840a833","https://dataon.kisti.re.kr/"]},
  {"source_id":"rda_agriculture_terms_crop","label":"농촌진흥청 농업용어/작물","license":"KR official public catalog metadata; API payload excluded","urls":["https://www.nongsaro.go.kr/portal/ps/psq/psqb/farmTermSimpleDicLst.ps?menuId=PS00064","https://www.data.go.kr/data/3061058/openapi.do"]},
  {"source_id":"mfds_food_nutrition_standard","label":"식약처 전국통합식품영양성분 표준데이터","license":"KR public standard data catalog metadata; nutrient payload excluded","urls":["https://www.data.go.kr/data/15100064/standard.do","https://various.foodsafetykorea.go.kr/nutrient/"]},
  {"source_id":"nibr_national_species_list","label":"국립생물자원관 국가생물종목록","license":"KR official public catalog metadata; species payload excluded","urls":["https://species.nibr.go.kr/","https://www.data.go.kr/data/15048041/fileData.do"]},
  {"source_id":"kobis_bioresource_catalog","label":"국가생명연구자원정보(KOBIS/KOBIC)","license":"KR official public catalog metadata; bioresource payload excluded","urls":["https://www.kobic.re.kr/","https://www.kribb.re.kr/kor/sub01/sub01_04_01_00_03.jsp"]},
  {"source_id":"seed_variety_names","label":"국립종자원 품종목록(공개 품종명)","license":"KR public catalog metadata; variety payload excluded","urls":["https://www.seed.go.kr/seed/index.do","https://www.seednet.go.kr/index.do","https://www.data.go.kr/data/15008652/fileData.do"]},
  {"source_id":"google_patent_phrase_similarity","label":"Google Patent Phrase Similarity","license":"CC-BY-4.0 dataset catalog metadata; phrase rows excluded","urls":["https://research.google/blog/announcing-the-patent-phrase-similarity-dataset/","https://www.kaggle.com/datasets/google/google-patent-phrase-similarity-dataset","https://www.kaggle.com/competitions/us-patent-phrase-to-phrase-matching/data"]},
  {"source_id":"rda_soil_heuktoram","label":"농촌진흥청 흙토람 토양정보","license":"KR official public catalog metadata; soil payload excluded","urls":["https://soil.rda.go.kr/sis/summary.do","https://soil.rda.go.kr/soil/index.jsp"]},
  {"source_id":"data_cdc_gov","label":"data.cdc.gov","license":"US public data catalog metadata; dataset payload excluded","urls":["https://data.cdc.gov/","https://data.cdc.gov/api/views.json?limit=50"]},
  {"source_id":"hhs_data_hub","label":"HHS Data Hub","license":"US public data catalog metadata; dataset payload excluded","urls":["https://healthdata.gov/","https://healthdata.gov/data.json"]},
  {"source_id":"kosis_portal","label":"KOSIS 국가통계포털","license":"KR official public catalog metadata; statistical payload excluded","urls":["https://kosis.kr/","https://kosis.kr/openapi/"]},
  {"source_id":"kosis_nass_agriculture_stats","label":"NASS류 KOSIS 농업통계","license":"KR official public catalog metadata; statistical payload excluded","urls":["https://kosis.kr/statHtml/statHtml.do?orgId=101&tblId=DT_1ET0010","https://kosis.kr/openapi/"]},
  {"source_id":"taas_traffic_accident_stats","label":"도로교통공단 TAAS 교통사고 집계","license":"KR official public catalog metadata; accident payload excluded","urls":["https://taas.koroad.or.kr/","https://taas.koroad.or.kr/web/shp/sbm/initTaas.do"]},
  {"source_id":"molit_araib_accident_refs","label":"국토부·항공철도사고조사위 집계/조사보고 ref","license":"KR official public catalog metadata; report bodies excluded","urls":["https://www.araib.molit.go.kr/","https://www.molit.go.kr/"]},
  {"source_id":"kca_product_safety_recall_meta","label":"한국소비자원·제품안전정보센터 리콜·위해 메타","license":"KR official public catalog metadata; recall payload excluded","urls":["https://www.consumer.go.kr/","https://www.safetykorea.kr/"]},
  {"source_id":"saferproducts_api","label":"SaferProducts.gov API","license":"US CPSC public catalog metadata; incident/recall payload excluded","urls":["https://www.saferproducts.gov/","https://www.saferproducts.gov/RestWebServices/Recall","https://www.cpsc.gov/Recalls"]},
  {"source_id":"ntsb_aviation_carol","label":"NTSB Aviation Accident/CAROL","license":"US federal public catalog metadata; accident payload excluded","urls":["https://data.ntsb.gov/carol-main-public/","https://www.ntsb.gov/Pages/AviationQueryV2.aspx"]},
  {"source_id":"ntsb_rail_carol","label":"NTSB Rail CAROL","license":"US federal public catalog metadata; rail accident payload excluded","urls":["https://data.ntsb.gov/carol-main-public/","https://www.ntsb.gov/investigations/AccidentReports/Pages/railroad.aspx"]},
  {"source_id":"dailymed_spl_catalog","label":"DailyMed SPL label catalog page metadata","license":"NLM public page metadata; SPL label prose/payload excluded","urls":["https://dailymed.nlm.nih.gov/dailymed/","https://dailymed.nlm.nih.gov/dailymed/services/v2/applicationdocs.json"]},
  {"source_id":"gnomad_catalog","label":"gnomAD public aggregate catalog metadata","license":"gnomAD public catalog metadata; aggregate/genotype payload excluded","urls":["https://gnomad.broadinstitute.org/","https://gnomad.broadinstitute.org/downloads"]},
  {"source_id":"thousand_genomes_catalog","label":"1000 Genomes public catalog metadata","license":"public/consent-scoped catalog metadata; genotype payload excluded","urls":["https://www.internationalgenome.org/","https://www.internationalgenome.org/data-portal/"]},
  {"source_id":"epa_toxcast_tox21_catalog","label":"EPA ToxCast/Tox21 assay catalog metadata","license":"US EPA public catalog metadata; assay result/payload excluded","urls":["https://www.epa.gov/chemical-research/toxicity-forecaster-toxcasttm-data","https://www.epa.gov/chemical-research/toxcast-and-tox21-summary-files"]},
  {"source_id":"mfds_drug_product_catalog","label":"식약처 의약품안전나라/통합정보 catalog metadata","license":"KR official public catalog metadata; drug product payload excluded","urls":["https://nedrug.mfds.go.kr/","https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15057639"]},
  {"source_id":"kcd_koicd_catalog","label":"KCD 한국표준질병사인분류 catalog metadata","license":"KR official public catalog metadata; diagnosis payload/advice excluded","urls":["https://www.koicd.kr/","https://kssc.kostat.go.kr/ksscNew_web/kssc/main/main.do?gubun=1"]},
  {"source_id":"kdca_infectious_stats_catalog","label":"질병관리청 감염병 집계통계 catalog metadata","license":"KR official public catalog metadata; epidemiology payload excluded","urls":["https://www.kdca.go.kr/","https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15043376"]},
  {"source_id":"molit_vehicle_recall_catalog","label":"국토부 자동차결함 리콜 catalog metadata","license":"KR official public catalog metadata; recall payload/advice excluded","urls":["https://www.car.go.kr/","https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15058946"]},
  {"source_id":"ts_vehicle_registration_inspection_catalog","label":"한국교통안전공단 자동차 등록·검사 catalog metadata","license":"KR official public catalog metadata; vehicle/owner payload excluded","urls":["https://www.kotsa.or.kr/","https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15004473"]},
  {"source_id":"kosha_industrial_accident_catalog","label":"고용노동부·KOSHA 산업재해 집계 catalog metadata","license":"KR official public catalog metadata; workplace/small-cell payload excluded","urls":["https://www.kosha.or.kr/","https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15043257"]},
  {"source_id":"safekorea_disaster_stats_catalog","label":"행정안전부·국민재난안전포털 재난 집계 catalog metadata","license":"KR official public catalog metadata; disaster payload/advice excluded","urls":["https://www.safekorea.go.kr/","https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15043334"]},
  {"source_id":"fda_caers_hfcs_catalog","label":"FDA CAERS/HFCS adverse event catalog metadata","license":"US federal public catalog metadata; adverse-event payload excluded","urls":["https://www.fda.gov/food/compliance-enforcement-food/cfsan-adverse-event-reporting-system-caers","https://www.fda.gov/food/food-safety-during-emergencies/harmful-algal-bloom-associated-illnesses"]},
  {"source_id":"fsis_recall_catalog","label":"FSIS recall catalog metadata","license":"USDA/FSIS public catalog metadata; recall payload/advice excluded","urls":["https://www.fsis.usda.gov/recalls","https://www.fsis.usda.gov/food-safety/recalls-public-health-alerts"]},
  {"source_id":"openfema_catalog","label":"OpenFEMA public dataset catalog metadata","license":"US federal public catalog metadata; disaster payload excluded","urls":["https://www.fema.gov/openfema-data-page","https://www.fema.gov/openfema-data-hub"]},
  {"source_id":"nhtsa_fars_catalog","label":"NHTSA FARS catalog metadata","license":"US federal public catalog metadata; crash case payload excluded","urls":["https://www.nhtsa.gov/research-data/fatality-analysis-reporting-system-fars","https://crashviewer.nhtsa.dot.gov/CrashAPI"]},
  {"source_id":"neiss_injury_catalog","label":"CPSC NEISS injury surveillance catalog metadata","license":"US federal public catalog metadata; injury case/narrative payload excluded","urls":["https://www.cpsc.gov/Research--Statistics/NEISS-Injury-Data","https://www.cpsc.gov/cgibin/NEISSQuery/home.aspx"]},
  {"source_id":"osha_ita_data_catalog","label":"OSHA Data/ITA injury catalog metadata","license":"US federal public catalog metadata; workplace injury payload excluded","urls":["https://www.osha.gov/Establishment-Specific-Injury-and-Illness-Data","https://www.osha.gov/data"]},
  {"source_id":"niosh_face_catalog","label":"NIOSH FACE fatality assessment catalog metadata","license":"US federal public catalog metadata; narrative report payload excluded","urls":["https://www.cdc.gov/niosh/face/","https://wwwn.cdc.gov/NIOSH-FACE/"]},
  {"source_id":"msha_accident_catalog","label":"MSHA Accident catalog metadata","license":"US federal public catalog metadata; Part-50 raw payload excluded","urls":["https://www.msha.gov/data-reports/accident-injuries","https://www.msha.gov/data-reports/mine-accident-injury-and-illness-report"]},
  {"source_id":"bls_iif_catalog","label":"BLS IIF injury statistics catalog metadata","license":"US federal public catalog metadata; statistics payload excluded","urls":["https://www.bls.gov/iif/","https://download.bls.gov/pub/time.series/ii/"]},
  {"source_id":"usfa_fire_nfirs_catalog","label":"USFA Fire/NFIRS catalog metadata","license":"US federal public catalog metadata; incident payload excluded","urls":["https://www.usfa.fema.gov/nfirs/","https://www.usfa.fema.gov/statistics/"]},
  {"source_id":"faa_wildlife_strike_catalog","label":"FAA Wildlife Strike Database catalog metadata","license":"US federal public catalog metadata; strike payload/advice excluded","urls":["https://wildlife.faa.gov/home","https://www.faa.gov/airports/airport_safety/wildlife"]},
  {"source_id":"bts_on_time_catalog","label":"BTS On-Time Performance catalog metadata","license":"US BTS public catalog metadata; flight row payload excluded","urls":["https://www.transtats.bts.gov/ONTIME/","https://www.transtats.bts.gov/DL_SelectFields.aspx?gnoyr_VQ=FGJ"]},
  {"source_id":"fra_accident_crossing_catalog","label":"FRA Accident/Crossing inventory catalog metadata","license":"US federal public catalog metadata; rail incident/crossing payload excluded","urls":["https://safetydata.fra.dot.gov/OfficeofSafety/publicsite/on_the_fly_download.aspx","https://safetydata.fra.dot.gov/OfficeofSafety/default.aspx"]},
  {"source_id":"fta_ntd_safety_catalog","label":"FTA NTD Safety catalog metadata","license":"US federal public catalog metadata; safety event payload excluded","urls":["https://www.transit.dot.gov/ntd/ntd-data","https://www.transit.dot.gov/ntd/data-product/safety-security-time-series-data"]},
  {"source_id":"uscg_marine_casualty_catalog","label":"USCG Marine Casualty & Pollution catalog metadata","license":"US federal public catalog metadata; casualty/pollution payload excluded","urls":["https://www.dco.uscg.mil/Our-Organization/Assistant-Commandant-for-Prevention-Policy-CG-5P/Inspections-Compliance-CG-5PC-/Office-of-Investigations-Casualty-Analysis/Marine-Casualty-and-Pollution-Data/","https://www.uscg.mil/"]},
  {"source_id":"uscg_boating_stats_catalog","label":"USCG Recreational Boating Statistics catalog metadata","license":"US federal public catalog metadata; boating accident payload excluded","urls":["https://uscgboating.org/statistics/accident_statistics.php","https://www.uscgboating.org/library/accident-statistics/"]},
  {"source_id":"epa_icis_npdes_catalog","label":"EPA ICIS-NPDES catalog metadata","license":"US EPA public catalog metadata; DMR/CEMS payload excluded","urls":["https://echo.epa.gov/tools/web-services","https://echo.epa.gov/tools/web-services/icis-npdes"]},
  {"source_id":"epa_attains_catalog","label":"EPA ATTAINS catalog metadata","license":"US EPA public catalog metadata; assessment payload excluded","urls":["https://www.epa.gov/waterdata/attains","https://attains.epa.gov/attains-public/api"]},
  {"source_id":"phmsa_pipeline_hazmat_catalog","label":"PHMSA Pipeline/Hazmat catalog metadata","license":"US federal public catalog metadata; incident/map payload excluded","urls":["https://www.phmsa.dot.gov/data-and-statistics/pipeline/data-and-statistics-overview","https://www.phmsa.dot.gov/hazmat-program-management-data-and-statistics/data-operations/incident-statistics"]},
  {"source_id":"openfda_device_clearance_recall_catalog","label":"openFDA 510(k)/PMA/Recall catalog metadata","license":"US federal public catalog metadata; device event/report payload excluded","urls":["https://open.fda.gov/apis/device/510k/","https://open.fda.gov/apis/device/pma/","https://open.fda.gov/apis/device/recall/"]},
  {"source_id":"eia_930_grid_monitor_catalog","label":"EIA-930 Grid Monitor catalog metadata","license":"US EIA public catalog metadata; hourly grid payload excluded","urls":["https://www.eia.gov/electricity/gridmonitor/","https://www.eia.gov/opendata/browser/electricity/rto"]},
  {"source_id":"ssa_open_data_catalog","label":"SSA Open Data catalog metadata","license":"US federal public catalog metadata; benefit/person payload excluded","urls":["https://www.ssa.gov/open/data/","https://www.ssa.gov/data/"]},
  {"source_id":"usa_gov_benefit_finder_catalog","label":"USA.gov Benefit Finder catalog metadata","license":"US federal public catalog metadata; eligibility advice/payload excluded","urls":["https://www.usa.gov/benefits","https://www.benefits.gov/"]},
  {"source_id":"cms_exchange_puf_catalog","label":"CMS Exchange PUF/MA-PartD catalog metadata","license":"US federal public catalog metadata; claims/CPT/person payload excluded","urls":["https://www.cms.gov/marketplace/resources/data/public-use-files","https://www.cms.gov/medicare/health-drug-plans/part-d-data"]},
  {"source_id":"daily_med_openfda_drug_label_block_ref","label":"DailyMed/openFDA drug-label blocked reference metadata","license":"mixed label catalog metadata only; label prose excluded","urls":["https://dailymed.nlm.nih.gov/dailymed/","https://open.fda.gov/apis/drug/label/"]}
]
JSON
python3 - "$OUT" "$UA" <<'PY'
import json,pathlib,subprocess,hashlib,datetime,sys,urllib.parse
out=pathlib.Path(sys.argv[1]); ua=sys.argv[2]
sources=json.loads((out/'sources.json').read_text())
for src in sources:
    sid=src['source_id']; d=out/'pages'/sid; d.mkdir(parents=True,exist_ok=True)
    results=[]
    for i,u in enumerate(src['urls']):
        ext='json' if any(x in u.lower() for x in ['api/','data.json','views.json']) else 'html'
        path=d/f'{i:02d}.{ext}'
        cmd=['curl','-L','--max-time','35','--retry','1','-A',ua,'-sS','-w','\nPNIX_HTTP_STATUS:%{http_code}\n',u]
        try:
            cp=subprocess.run(cmd,text=True,capture_output=True,timeout=45)
            txt=cp.stdout
            status='000'
            marker='PNIX_HTTP_STATUS:'
            if marker in txt:
                body,tail=txt.rsplit(marker,1); status=tail.strip().split()[0]; txt=body
            path.write_text(txt,encoding='utf-8',errors='ignore')
            results.append({'url':u,'path':str(path.relative_to(out)),'http_status':status,'curl_exit':cp.returncode,'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
        except Exception as e:
            fail=d/f'{i:02d}.error.txt'; fail.write_text(str(e),encoding='utf-8')
            results.append({'url':u,'path':str(fail.relative_to(out)),'http_status':'000','curl_exit':999,'error':str(e),'bytes':fail.stat().st_size,'sha256':hashlib.sha256(fail.read_bytes()).hexdigest()})
    src['fetch_results']=results
files=[]
for p in sorted(out.rglob('*')):
    if p.is_file() and p.name!='source-manifest.json':
        files.append({'path':str(p.relative_to(out)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
manifest={'schema':'pnix.source_manifest.v1','source_id':'remaining-allowlist-public-catalogs','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Official public catalog/page/API metadata only; payload records/prose bodies/credentials/advice/graph wiring excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
