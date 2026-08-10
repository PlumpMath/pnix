#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NIST_LEGAL_METROLOGY_DEST:-$ROOT/ingest/units/nist-legal-metrology-handbooks}"
UA="${NIST_LEGAL_METROLOGY_USER_AGENT:-pnix-ingest/0.1 (NIST legal metrology handbook metadata catalog)}"
mkdir -p "$DEST/raw"
curl -fsSL -A "$UA" "https://www.nist.gov/pml/owm/nist-handbook-44-current-edition" -o "$DEST/raw/handbook-44-current.html"
curl -fsSL -A "$UA" "https://www.nist.gov/pml/owm/nist-handbook-130-current-edition" -o "$DEST/raw/handbook-130-current.html"
curl -fsSL -A "$UA" "https://www.nist.gov/pml/owm/owm-products-and-services/publications-and-documentary-standards/handbooks" -o "$DEST/raw/handbooks-index.html"
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*.html')):
    b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'units.nist_legal_metrology_handbooks.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / NIST public publication metadata','files':files,'excluded':['PDF/DOCX body text','tolerance tables','inspection procedures','legal/regulatory advice','device pass/fail decisions','state adoption interpretation','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched NIST legal metrology handbook pages files={len(files)} into {root}')
PY
