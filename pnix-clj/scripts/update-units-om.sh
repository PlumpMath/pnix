#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${OM_DEST:-$ROOT/ingest/units/om}"
URL="${OM_RDF_URL:-https://raw.githubusercontent.com/HajoRijgersberg/OM/master/om-2.0.rdf}"
USER_AGENT="${OM_USER_AGENT:-pnix-ingest/0.1 (OM bounded structural ingest)}"
mkdir -p "$DEST/raw"
echo "OM RDF/XML 수집: $URL" >&2
curl -fsSL -A "$USER_AGENT" "$URL" -o "$DEST/raw/om-2.0.rdf"
python3 - <<'PY' "$DEST" "$URL"
import hashlib, json, pathlib, sys, time
root=pathlib.Path(sys.argv[1]); url=sys.argv[2]
files=[]
for p in sorted((root/'raw').glob('*')):
    if p.is_file():
        b=p.read_bytes(); files.append({'path':str(p.relative_to(root)), 'sha256':hashlib.sha256(b).hexdigest(), 'bytes':len(b)})
receipt={
  'schema':'units.om.source_receipt.v1',
  'source':'OM — Ontology of units of Measure',
  'url':url,
  'retrieved_at_unix':int(time.time()),
  'license':'CC-BY-4.0',
  'included_files':['om-2.0.rdf'],
  'excluded_files':['om-2-ucum.ttl'],
  'excluded_content':['comments','descriptions','examples','reasoner entailments','mirror graph wiring'],
  'files':files,
}
(root/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
PY
echo "완료: $DEST" >&2
