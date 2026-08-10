#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/catalogue-of-life-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (COL/ChecklistBank API catalog metadata only; no taxon dumps)"
BASE="${COL_CHECKLISTBANK_API_BASE:-https://api.checklistbank.org}"
curl -fsSL --retry 2 --max-time 60 -A "$UA" "$BASE/dataset?limit=${COL_DATASET_LIMIT:-25}" -o "$OUT/datasets.json"
curl -fsSL --retry 2 --max-time 60 -A "$UA" "$BASE/dataset?limit=${COL_DATASET_LIMIT:-25}&q=Catalogue%20of%20Life" -o "$OUT/col-datasets.json"
curl -fsSL --retry 2 --max-time 60 -A "$UA" "$BASE/vocab/rank" -o "$OUT/ranks.json"
curl -fsSL --retry 2 --max-time 60 -A "$UA" "$BASE/openapi.json" -o "$OUT/openapi.json"
python3 - "$OUT" "$BASE" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); base=sys.argv[2]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'catalogue-of-life-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':[base+'/dataset?limit=25',base+'/dataset?limit=25&q=Catalogue%20of%20Life',base+'/vocab/rank',base+'/openapi.json'],'files':files,'policy':'COL/ChecklistBank API catalog metadata only; taxon/name payloads, full dumps, citation HTML/prose, credentials and graph wiring excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
