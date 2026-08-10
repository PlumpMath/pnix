#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/regulatory/govinfo-api"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (documentation metadata only; contact: local)"
DOCS_URL="${GOVINFO_DOCS_URL:-https://api.govinfo.gov/docs/}"
FEATURE_URL="${GOVINFO_FEATURE_URL:-https://www.govinfo.gov/features/api}"
REPO_API_URL="${GOVINFO_REPO_API_URL:-https://api.github.com/repos/usgpo/api}"
curl -fsSL -A "$UA" "$DOCS_URL" -o "$OUT/api-docs.html"
curl -fsSL -A "$UA" "$FEATURE_URL" -o "$OUT/feature-api.html"
curl -fsSL -A "$UA" "$REPO_API_URL" -o "$OUT/github-repo.json"
python3 - "$OUT" "$DOCS_URL" "$FEATURE_URL" "$REPO_API_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'govinfo-api','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'documentation shell and endpoint catalog tokens only; API-key live package data/full text/legal judgment excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
