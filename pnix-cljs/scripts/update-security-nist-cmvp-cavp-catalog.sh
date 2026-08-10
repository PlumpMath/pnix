#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/security/nist-cmvp-cavp-catalog"
mkdir -p "$OUT"
URLS=(
  "${NIST_CMVP_URL:-https://csrc.nist.gov/projects/cryptographic-module-validation-program}"
  "${NIST_CMVP_MODULES_URL:-https://csrc.nist.gov/projects/cryptographic-module-validation-program/validated-modules}"
  "${NIST_CAVP_URL:-https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program}"
)
idx=0
for url in "${URLS[@]}"; do
  idx=$((idx+1))
  curl -L --fail --max-time 45 --retry 2 --retry-delay 1 -A "pnix-ingest/1.0" "$url" -o "$OUT/page-$idx.html"
  printf '%s\t%s\n' "page-$idx.html" "$url" >> "$OUT/urls.tsv.tmp"
done
mv "$OUT/urls.tsv.tmp" "$OUT/urls.tsv"
python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1])
files=[]
for p in sorted(out.glob('page-*.html')):
    b=p.read_bytes(); files.append({'path':p.name,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
urls=[]
for line in (out/'urls.tsv').read_text().splitlines():
    f,u=line.split('\t',1); urls.append({'file':f,'url':u})
manifest={'schema':'pnix.ingest.source_manifest.v1','source':'NIST CMVP/CAVP catalog metadata','retrieved_at':datetime.date.today().isoformat(),'source_urls':urls,'files':files,'policy':'official CSRC page catalog metadata only; cert rows/security judgments/test vectors excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
