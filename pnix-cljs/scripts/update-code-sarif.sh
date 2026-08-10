#!/usr/bin/env bash
# SARIF 2.1.0 official JSON schema updater.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/sarif"
mkdir -p "$DEST"
URL="${SARIF_SCHEMA_URL:-https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json}"
OUT="$DEST/sarif-schema-2.1.0.json"
TMP="$OUT.tmp"
curl -fsSL "$URL" -o "$TMP"
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
mv "$TMP" "$OUT"
python3 - "$DEST/source-receipt.json" "$URL" "$SHA" "$OUT" <<'PY'
import json,sys,datetime,os
out,url,sha,path=sys.argv[1:]
data=json.load(open(path))
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'SARIF 2.1.0 JSON Schema',
  'version':'2.1.0',
  'url':url,
  'sha256':sha,
  'size_bytes':os.path.getsize(path),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'title':data.get('title'),
  'json_schema':data.get('$schema'),
  'license':'OASIS specification / publicly available standard',
  'scope':'JSON schema structure only; prose descriptions, actual logs, source locations, and autofix payloads excluded'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
echo "updated SARIF schema: $SHA"
