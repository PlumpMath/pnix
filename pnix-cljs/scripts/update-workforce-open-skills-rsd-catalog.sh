#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/workforce/open-skills-rsd-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (RSD doc catalog metadata only; no skill payloads)"
curl -fsSL --retry 2 --max-time 45 -A "$UA" "${OPEN_SKILLS_RSD_URL:-https://rsd.openskillsnetwork.org/}" -o "$OUT/rsd.html"
curl -fsSL --retry 2 --max-time 45 -A "$UA" "${OPEN_SKILLS_OSMT_PAGE_URL:-https://www.openskillsnetwork.org/osmt}" -o "$OUT/osmt-page.html"
curl -fsSL --retry 2 --max-time 45 -A "$UA" "${OPEN_SKILLS_OSMT_REPO_API:-https://api.github.com/repos/wgu-opensource/osmt}" -o "$OUT/osmt-repo.json"
python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1])
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'open-skills-rsd-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'files':files,'policy':'RSD documentation metadata only; OSMT source/data and skill payloads excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
