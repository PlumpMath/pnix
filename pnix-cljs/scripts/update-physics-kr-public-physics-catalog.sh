#!/usr/bin/env bash
# Korean public physics-related official data.go.kr pages -> local raw metadata snapshot.
# Raw pages stay under gitignored ingest/; generated redb row stores catalog metadata only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${KR_PUBLIC_PHYSICS_OUT:-$ROOT/ingest/physics/kr-public-physics-catalog}"
UA="pnix-ingest/1.0 (Korean public physics catalog metadata only; no content payload)"
mkdir -p "$OUT/pages"
cat > "$OUT/sources.json" <<'JSON'
[
  {"source_id":"data_go_kr_kofac_smart_science_math_lab","label":"한국과학창의재단 스마트 수과학실 catalog","license":"Data.go.kr public catalog metadata; program/content payload excluded","urls":["https://www.data.go.kr/data/15121150/fileData.do"]},
  {"source_id":"data_go_kr_nsm_science_learning_catalog","label":"국립중앙과학관 과학학습콘텐츠 catalog","license":"Data.go.kr public catalog metadata; third-party content/prose excluded","urls":["https://www.data.go.kr/data/15067815/fileData.do"]},
  {"source_id":"data_go_kr_kofac_excellent_science_books","label":"한국과학창의재단 우수과학도서 정보 catalog","license":"Data.go.kr public catalog metadata; book content excluded","urls":["https://www.data.go.kr/data/15017851/fileData.do"]},
  {"source_id":"data_go_kr_nsm_science_fair_awards","label":"국립중앙과학관 수상작정보 catalog","license":"Data.go.kr public catalog metadata; award work bodies and personal payload excluded","urls":["https://www.data.go.kr/data/15135618/fileData.do"]},
  {"source_id":"data_go_kr_youth_space_center_spectrum","label":"국립청소년우주센터 천문관측 분광 catalog","license":"Data.go.kr public catalog metadata; spectrum images/raw payload excluded","urls":["https://www.data.go.kr/data/15102072/fileData.do"]},
  {"source_id":"data_go_kr_kocw_course_catalog","label":"KOCW 공개강의서비스정보 catalog","license":"Data.go.kr public standard-data catalog metadata; course media/prose excluded","urls":["https://www.data.go.kr/data/15107732/standard.do"]},
  {"source_id":"data_go_kr_nrf_kci_journal_catalog","label":"한국연구재단 KCI학술지정보 catalog","license":"Data.go.kr public catalog metadata; article/abstract/citation payload excluded","urls":["https://www.data.go.kr/data/3049043/fileData.do"]},
  {"source_id":"data_go_kr_kirams_paper_catalog","label":"한국원자력의학원 논문정보 catalog","license":"Data.go.kr public catalog metadata; paper/person payload excluded","urls":["https://www.data.go.kr/data/15047482/fileData.do"]},
  {"source_id":"data_go_kr_kofac_sciencetimes_scitech","label":"한국과학창의재단 사이언스타임즈 과학기술 catalog","license":"Data.go.kr public catalog metadata; article body excluded","urls":["https://www.data.go.kr/data/15093598/fileData.do"]}
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
manifest={'schema':'pnix.source_manifest.v1','source_id':'kr-public-physics-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'sources':sources,'files':files,'policy':'Korean official public-data physics/science catalog metadata only; no instructional prose, no images, no course media, no experiment procedures, no problems/solutions, no article payload, no graph wiring.'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'sources':len(sources),'files':len(files),'out':str(out)},ensure_ascii=False))
PY
