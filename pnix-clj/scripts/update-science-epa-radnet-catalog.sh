#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/epa-radnet-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
RADNET_URL="${EPA_RADNET_URL:-https://www.epa.gov/radnet}"
AIR_URL="${EPA_RADNET_AIR_URL:-https://www.epa.gov/radnet/radnet-near-real-time-air-data}"
curl -fsSL -A "$UA" "$RADNET_URL" -o "$OUT/radnet.html"
curl -fsSL -A "$UA" "$AIR_URL" -o "$OUT/near-real-time-air.html"
python3 - "$OUT" "$RADNET_URL" "$AIR_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'epa-radnet-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'EPA RadNet catalog pages only; measurements/dose/exposure/radiation-safety decisions excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
