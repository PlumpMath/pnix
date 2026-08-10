#!/usr/bin/env bash
# NRC operating power reactors official workbook -> local raw snapshot.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NRC_POWER_REACTORS_DEST:-$ROOT/ingest/science/nrc-power-reactors}"
URL="${NRC_POWER_REACTORS_URL:-https://www.nrc.gov/reading-rm/doc-collections/datasets/reactors-operating.xls}"
mkdir -p "$DEST"
OUT="$DEST/reactors-operating.xls"
curl --http1.1 -fsSL --retry 2 --retry-delay 3 --connect-timeout 10 --max-time 45 -A 'pnix-nrc-power-reactors-ingest' "$URL" -o "$OUT"
python3 - "$URL" "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys
url=sys.argv[1]; dest=pathlib.Path(sys.argv[2])
out=dest/'reactors-operating.xls'
raw=out.read_bytes()
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'NRC Commercial Nuclear Power Reactors – Operating Reactors dataset',
  'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),
  'source_url':url,
  'source_pages':['https://www.nrc.gov/reading-rm/doc-collections/datasets/index.html','https://www.nrc.gov/data/index','https://www.nrc.gov/site-help/disclaimer'],
  'license':'NRC U.S. Government Work public-domain / no copyright; courtesy credit requested',
  'scope':'official reactors-operating.xlsx only; raw location text, precise coordinates, capacity/current status, operational/security/emergency guidance, and graph/mirror wiring excluded by generator',
  'sha256':hashlib.sha256(raw).hexdigest(),
  'size_bytes':len(raw),
}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded NRC power reactors workbook: bytes={len(raw)} sha256={receipt["sha256"]} -> {out}')
PY
