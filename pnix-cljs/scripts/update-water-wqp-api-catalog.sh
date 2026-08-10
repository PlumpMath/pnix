#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/water/wqp-api-catalog"
mkdir -p "$OUT"
BASE_URL="${WQP_BASE_URL:-https://www.waterqualitydata.us/}"
DOC_URL="${WQP_DOC_URL:-https://www.waterqualitydata.us/webservices_documentation/}"
fetch() {
  local url="$1" out="$2"
  curl -L --fail --max-time 40 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$url" -o "$out"
}
fetch "$BASE_URL" "$OUT/home.html"
fetch "$DOC_URL" "$OUT/webservices_documentation.html"
python3 - "$OUT" "$BASE_URL" "$DOC_URL" <<'PY'
import hashlib, json, pathlib, sys, datetime
out=pathlib.Path(sys.argv[1])
urls=sys.argv[2:]
files=[]
for p in sorted(out.glob('*.html')):
    b=p.read_bytes()
    files.append({'path':p.name,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'Water Quality Portal API catalog metadata','retrieved_at':datetime.date.today().isoformat(),'source_urls':urls,'files':files,'policy':'official web-service catalog metadata only; monitoring rows/payload values excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
