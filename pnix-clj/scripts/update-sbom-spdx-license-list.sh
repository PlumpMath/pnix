#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${SPDX_LICENSE_LIST_DEST:-$ROOT/ingest/sbom/spdx-license-list}"
UA="${SPDX_LICENSE_LIST_USER_AGENT:-pnix-ingest/0.1 (SPDX License List identifier metadata)}"
mkdir -p "$DEST/raw"
curl -fsSL -A "$UA" "${SPDX_LICENSES_URL:-https://raw.githubusercontent.com/spdx/license-list-data/main/json/licenses.json}" -o "$DEST/raw/licenses.json"
curl -fsSL -A "$UA" "${SPDX_EXCEPTIONS_URL:-https://raw.githubusercontent.com/spdx/license-list-data/main/json/exceptions.json}" -o "$DEST/raw/exceptions.json"
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*.json')):
    b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'sbom.spdx_license_list.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'CC-BY / SPDX project terms','files':files,'excluded':['full license texts','matching templates','legal interpretation','compatibility/compliance judgments','customer SBOM documents','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched SPDX license-list JSON files={len(files)} into {root}')
PY
