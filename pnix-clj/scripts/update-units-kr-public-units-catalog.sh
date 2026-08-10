#!/usr/bin/env bash
# Korean public unit-related official source pages -> local raw metadata snapshot.
# Stores raw pages only under gitignored ingest/; generated redb row stores facts/catalog metadata only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_UNITS_OUT:-$ROOT/ingest/units/kr-public-units-catalog}"
UA="pnix-ingest/1.0 (Korean public unit catalog metadata only; no legal prose/payload rows)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"kats_legal_units_overview","label":"국가기술표준원 법정단위 개요","license":"KATS official legal-unit page metadata; facts-only", "urls":["https://www.kats.go.kr/content.do?cmsid=75"]},
  {"source_id":"law_measurement_act","label":"계량에 관한 법률 source reference","license":"Korean legal source reference; statute text excluded", "urls":["https://www.law.go.kr/lsInfoP.do?lsiSeq=136928"]},
  {"source_id":"law_national_standards_framework_decree","label":"국가표준기본법 시행령 source reference","license":"Korean legal source reference; statute text excluded", "urls":["https://www.law.go.kr/lsInfoP.do?lsiSeq=205698"]},
  {"source_id":"data_go_kr_agri_trade_unit_mapping_api","label":"농수축산물 거래단량 매핑 정보조회 catalog","license":"Data.go.kr public API catalog metadata; API payload excluded", "urls":["https://www.data.go.kr/data/15109065/openapi.do"]},
  {"source_id":"data_go_kr_agri_standard_unit_mapping_file","label":"농수축산물 표준코드-조사가격단위매핑목록 catalog","license":"Data.go.kr public file catalog metadata; file payload excluded unless separately gated", "urls":["https://www.data.go.kr/data/15045738/fileData.do"]},
  {"source_id":"data_go_kr_kats_national_standard_catalog","label":"국가기술표준원 이나라표준인증 국가표준 catalog","license":"Data.go.kr public catalog metadata; KS document bodies excluded", "urls":["https://www.data.go.kr/data/15131276/fileData.do"]}
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-units-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean official units/legal/public-data catalog metadata only; no full statute text, no standards body text, no PDF/HWP body, no API payload rows, no advice, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
