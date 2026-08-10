#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NIST_SP800_63_DEST:-$ROOT/ingest/security/nist-sp800-63-assurance-levels}"
UA="${NIST_SP800_63_USER_AGENT:-pnix-ingest/0.1 (NIST SP800-63 assurance-level metadata catalog)}"
mkdir -p "$DEST/raw"
for doc in index sp800-63 sp800-63a sp800-63b sp800-63c; do
  url="https://pages.nist.gov/800-63-4/"
  [ "$doc" = "index" ] || url="https://pages.nist.gov/800-63-4/${doc}.html"
  curl -fsSL -A "$UA" "$url" -o "$DEST/raw/${doc}.html"
done
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*.html')):
    b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'security.nist_sp800_63.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / NIST public publication metadata','files':files,'excluded':['requirement prose','risk assessment decisions','compliance advice','authorization/security policy','prod auth logs','privilege escalation/bypass procedures','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched NIST SP800-63 pages files={len(files)} into {root}')
PY
