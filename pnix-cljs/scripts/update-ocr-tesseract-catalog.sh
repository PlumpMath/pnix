#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/ocr/tesseract-catalog"
REF="${TESSERACT_REF:-main}"
mkdir -p "$DST"
if ! curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/tesseract-ocr/tesseract/git/trees/$REF?recursive=1"; then
  test -s "$DST/tree.json" && echo "WARN: tree download failed; reusing existing $DST/tree.json" >&2 || exit 1
fi
if ! curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "https://raw.githubusercontent.com/tesseract-ocr/tesseract/$REF/LICENSE"; then
  test -s "$DST/LICENSE" && echo "WARN: license download failed; reusing existing $DST/LICENSE" >&2 || exit 1
fi
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for name in ['tree.json','LICENSE']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'tesseract-catalog','source_name':'Tesseract OCR source catalog','license_id':'Apache-2.0','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/tesseract-ocr/tesseract','https://api.github.com/repos/tesseract-ocr/tesseract/git/trees/'+ref+'?recursive=1'],'files':files,'policy':'Git tree path metadata only. Exclude source bodies, docs prose, tests, traineddata/model files, images/PDFs, OCR outputs, runtime execution, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} files={len(files)}')
PY
