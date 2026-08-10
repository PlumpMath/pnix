#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${SBOM_SCHEMA_DEST:-$ROOT/ingest/sbom/format-schemas}"
UA="${SBOM_SCHEMA_USER_AGENT:-pnix-ingest/0.1 (SBOM format schema metadata)}"
mkdir -p "$DEST/raw"
curl -fsSL -A "$UA" "${CYCLONEDX_SCHEMA_URL:-https://raw.githubusercontent.com/CycloneDX/specification/master/schema/bom-1.7.schema.json}" -o "$DEST/raw/cyclonedx-bom.schema.json"
curl -fsSL -A "$UA" "${SPDX_SCHEMA_URL:-https://spdx.org/schema/3.0.1/spdx-json-schema.json}" -o "$DEST/raw/spdx-json-schema.json"
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*.json')):
    b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'sbom.format_schemas.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'CycloneDX Apache-2.0; SPDX public schema metadata','files':files,'excluded':['actual SBOM documents','customer/proprietary inventory','dependency graph facts','vulnerability scan results','license compliance decisions','package source code','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched SBOM schema files={len(files)} into {root}')
PY
