#!/usr/bin/env bash
# Korean public math-related official data.go.kr pages -> local raw metadata snapshot.
# Raw pages stay under gitignored ingest/; generated redb row stores catalog metadata only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_MATH_OUT:-$ROOT/ingest/math/kr-public-math-catalog}"
UA="pnix-ingest/1.0 (Korean public math catalog metadata only; no PDFs/problems/solutions/prose payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"data_go_kr_kofac_math_curriculum_api","label":"한국과학창의재단 수학교육과정 교수학습자료 OpenAPI catalog","license":"Data.go.kr public API catalog metadata; API payload/PDF content excluded","urls":["https://www.data.go.kr/data/15042056/openapi.do"]},
  {"source_id":"data_go_kr_kofac_math_curriculum_file","label":"한국과학창의재단 수학교육과정 교수학습자료 file catalog","license":"Data.go.kr public file catalog metadata; PDF bodies excluded","urls":["https://www.data.go.kr/data/15083209/fileData.do"]},
  {"source_id":"data_go_kr_kofac_askmath_play_catalog","label":"한국과학창의재단 AskMath 수학놀이 콘텐츠 catalog","license":"Data.go.kr public catalog metadata; video/body payload excluded","urls":["https://www.data.go.kr/data/15093531/fileData.do"]},
  {"source_id":"data_go_kr_kofac_askmath_contest_catalog","label":"한국과학창의재단 AskMath 공모전 자료 catalog","license":"Data.go.kr public catalog metadata; submission bodies excluded","urls":["https://www.data.go.kr/data/15093572/fileData.do"]},
  {"source_id":"data_go_kr_kofac_askmath_report_catalog","label":"한국과학창의재단 AskMath 연구보고서 catalog","license":"Data.go.kr public catalog metadata; report bodies excluded","urls":["https://www.data.go.kr/data/15093603/fileData.do"]},
  {"source_id":"data_go_kr_nsm_science_learning_catalog","label":"국립중앙과학관 과학학습콘텐츠 catalog","license":"Data.go.kr public catalog metadata; third-party content/prose excluded","urls":["https://www.data.go.kr/data/15067815/fileData.do"]},
  {"source_id":"data_go_kr_nrf_kci_journal_catalog","label":"한국연구재단 KCI학술지정보 catalog","license":"Data.go.kr public catalog metadata; article/abstract/citation payload excluded","urls":["https://www.data.go.kr/data/3049043/fileData.do"]}
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-math-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean official public-data math catalog metadata only; no PDFs, no instructional prose, no contest/report bodies, no videos, no problems/solutions, no third-party encyclopedia content, no article payload, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
