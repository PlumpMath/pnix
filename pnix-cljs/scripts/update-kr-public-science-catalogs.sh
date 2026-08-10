#!/usr/bin/env bash
# Korean official public science/environment catalog pages -> local raw metadata snapshot.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_SCIENCE_OUT:-$ROOT/ingest/korea/kr-public-science-catalogs}"
UA="pnix-ingest/1.0 (Korean public science catalog metadata only; no payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kr_kma_asos_daily_api","domain":"weather","label":"기상청 지상(종관, ASOS) 일자료 조회서비스 catalog","license":"Data.go.kr public catalog metadata; observation payload excluded","urls":["https://www.data.go.kr/data/15059093/openapi.do"],"declared_field_refs":["station id","date/time request parameter","temperature field name","humidity field name","weather observation output element names"],"excluded_payload":"weather observation values, forecasts, warnings, operational decisions"},
  {"source_id":"kr_kma_earthquake_info_api","domain":"seismic","label":"기상청 지진정보 조회서비스 catalog","license":"Data.go.kr public catalog metadata; event payload excluded","urls":["https://www.data.go.kr/data/15000420/openapi.do"],"declared_field_refs":["origin time field name","epicenter location field name","magnitude field name","intensity field name","tsunami-related output element names"],"excluded_payload":"earthquake event rows, warning messages, response guidance"},
  {"source_id":"kr_kma_earthquake_observation_environment","domain":"seismic","label":"기상청 지진관측환경 catalog","license":"Data.go.kr public catalog metadata; station metadata payload excluded","urls":["https://www.data.go.kr/data/15133541/fileData.do"],"declared_field_refs":["station metadata file reference","observation place field name","equipment metadata field name","NECIS reference"],"excluded_payload":"station raw rows, precise equipment payload, operational monitoring guidance"},
  {"source_id":"kr_khoa_tide_observed_predicted_api","domain":"oceanography","label":"국립해양조사원 조위관측소 실측·예측 조위 조회 catalog","license":"Data.go.kr public catalog metadata; tide payload excluded","urls":["https://www.data.go.kr/data/15142507/openapi.do"],"declared_field_refs":["station code parameter","observation date parameter","station name field name","latitude/longitude field names","observed/predicted tide field names"],"excluded_payload":"tide observations/predictions, navigation or port operation guidance"},
  {"source_id":"kr_khoa_latest_ocean_station_api","domain":"oceanography","label":"국립해양조사원 조위관측소 최신 관측데이터 catalog","license":"Data.go.kr public catalog metadata; latest observation payload excluded","urls":["https://www.data.go.kr/data/15155508/openapi.do"],"declared_field_refs":["station code parameter","time interval parameter","water temperature field name","salinity field name","wind/current field names"],"excluded_payload":"latest ocean observations, maritime safety decisions"},
  {"source_id":"kr_khoa_buoy_latest_api","domain":"oceanography","label":"국립해양조사원 해양관측부이 최신 관측데이터 catalog","license":"Data.go.kr public catalog metadata; buoy payload excluded","urls":["https://www.data.go.kr/data/15155516/openapi.do"],"declared_field_refs":["buoy/station code parameter","wave height field name","water temperature field name","wind/current field names"],"excluded_payload":"buoy time-series values, forecasts, navigation guidance"},
  {"source_id":"kr_khoa_sea_fog_observation_api","domain":"ocean_weather","label":"국립해양조사원 해무관측소 관측 데이터 조회 catalog","license":"Data.go.kr public catalog metadata; fog observation payload excluded","urls":["https://www.data.go.kr/data/15142519/openapi.do"],"declared_field_refs":["fog station code parameter","visibility field name","humidity field name","temperature/pressure/wind field names"],"excluded_payload":"fog observations, shipping/port operation decisions"},
  {"source_id":"kr_kwater_flow_station_coordinates","domain":"hydrology","label":"한국수자원공사 유량측정소 좌표 정보 catalog","license":"Data.go.kr public catalog metadata; coordinate payload excluded","urls":["https://www.data.go.kr/data/15126361/fileData.do"],"declared_field_refs":["flow station name field","latitude field","longitude field","river name field","management agency field"],"excluded_payload":"full coordinate rows, real-time flow values, flood operation guidance"},
  {"source_id":"kr_kwater_hydraulic_structures","domain":"hydrology","label":"한국수자원공사 수문제원현황 catalog","license":"Data.go.kr public catalog metadata; structure payload excluded","urls":["https://www.data.go.kr/data/15083336/fileData.do"],"declared_field_refs":["dam/weir type field","height/length field names","storage capacity field names","basin area field name","spillway elevation field name"],"excluded_payload":"dam/weir structure rows, engineering suitability or operation guidance"},
  {"source_id":"kr_kwater_groundwater_survey_facilities","domain":"hydrogeology","label":"한국수자원공사 지하수 기초 조사 시설 조회 catalog","license":"Data.go.kr public catalog metadata; facility payload excluded","urls":["https://www.data.go.kr/data/15104454/fileData.do"],"declared_field_refs":["facility class field","observation facility name field","address field name","source institution field name"],"excluded_payload":"facility rows, addresses as payload, groundwater safety or permitting guidance"},
  {"source_id":"kr_airkorea_air_quality_api","domain":"atmosphere","label":"한국환경공단 에어코리아 대기오염정보 catalog","license":"Data.go.kr public catalog metadata; measurement payload excluded","urls":["https://www.data.go.kr/tcs/dss/selectApiDataDetailView.do?publicDataPk=15073861"],"declared_field_refs":["station measurement query feature","city/province query feature","PM10 field name","PM2.5 field name","O3/NO2/CO/SO2 field names"],"excluded_payload":"real-time air-quality values, forecasts, health or operational guidance"},
  {"source_id":"kr_airkorea_confirmed_measurements_catalog","domain":"atmosphere","label":"에어코리아 최종확정 측정자료 catalog","license":"Data.go.kr public catalog metadata; XLSX payload excluded","urls":["https://www.data.go.kr/data/15122830/fileData.do"],"declared_field_refs":["station code field","station name field","measurement timestamp field","SO2/CO/O3/NO2/PM10/PM25 field names"],"excluded_payload":"XLSX rows, pollutant values, public-health guidance"},
  {"source_id":"kr_kigam_geologic_map_search","domain":"geology","label":"한국지질자원연구원 지오빅데이터 지질주제도 통합검색 catalog","license":"Data.go.kr public catalog metadata; map/image payload excluded","urls":["https://www.data.go.kr/data/15081769/fileData.do"],"declared_field_refs":["geologic thematic map service URL","keyword field names","map/image media type field name"],"excluded_payload":"map images, GIS layers, geologic interpretation or engineering advice"},
  {"source_id":"kr_keiti_soil_groundwater_rd_catalog","domain":"soil_groundwater","label":"한국환경산업기술원 토양지하수 개발기술현황 catalog","license":"Data.go.kr public catalog metadata; project payload excluded","urls":["https://www.data.go.kr/data/15088158/fileData.do"],"declared_field_refs":["project name field","business unit field","research status field","lead institution field"],"excluded_payload":"project rows, technology prose, remediation instructions, environmental compliance advice"},
  {"source_id":"kr_forest_landslide_history_catalog","domain":"geohazard","label":"산림청 최근 5년간 전국 산사태 발생 이력 catalog","license":"Data.go.kr public catalog metadata; event payload excluded","urls":["https://www.data.go.kr/data/15125006/fileData.do"],"declared_field_refs":["damage cause field","damage address field","damage area field","event time/location field names"],"excluded_payload":"landslide event rows, addresses, risk maps, emergency response guidance"},
  {"source_id":"kr_forest_fire_statistics_api","domain":"wildfire","label":"산림청 산불발생통계 catalog","license":"Data.go.kr public catalog metadata; statistics payload excluded","urls":["https://www.data.go.kr/data/3070842/openapi.do"],"declared_field_refs":["region field","cause field","occurrence count field","damaged area field","suppression time/equipment field names"],"excluded_payload":"wildfire statistics rows, forecasts, suppression or resource allocation guidance"}
]
JSON
python3 - "$OUT" "$UA" <<'PY'
import datetime, hashlib, json, pathlib, subprocess, sys
out=pathlib.Path(sys.argv[1]); ua=sys.argv[2]
sources=json.loads((out/'sources.json').read_text(encoding='utf-8'))
for src in sources:
    sid=src['source_id']; d=out/'pages'/sid; d.mkdir(parents=True, exist_ok=True)
    results=[]
    for i,u in enumerate(src['urls']):
        path=d/f'{i:02d}.html'
        cmd=['curl','-L','--max-time','35','--retry','1','-A',ua,'-sS','-w','\nPNIX_HTTP_STATUS:%{http_code}\n',u]
        try:
            cp=subprocess.run(cmd,text=True,capture_output=True,timeout=45)
            txt=cp.stdout; status='000'; marker='PNIX_HTTP_STATUS:'
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-science-catalogs','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean official public science/environment catalog metadata only; no dataset rows, no observations, no forecasts, no warnings, no maps/geometries, no prose bodies, no operational guidance, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
