#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OPTIMADE_SCHEMA_OUT:-$ROOT/ingest/materials/optimade-schema-catalog}"
REF="${OPTIMADE_REF:-v1.2.0}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/materials/LICENSES/optimade-schema-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "optimade-schema-catalog" "$receipt"
python3 - "$OUT" "$REF" <<'PY'
import json, sys, urllib.request
from pathlib import Path
out=Path(sys.argv[1]); ref=sys.argv[2]
base='https://api.github.com/repos/Materials-Consortia/OPTIMADE'
def get(url):
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/metadata-only'})
    return json.load(urllib.request.urlopen(req, timeout=30))
schemas=get(f'{base}/contents/schemas?ref={ref}')
src=[x for x in schemas if x.get('path')=='schemas/src'][0]
tree=get(src['git_url']+'?recursive=1')
meta={'schema':'materials.optimade_schema_catalog.raw.v1','retrieved_at':'2026-06-20','ref':ref,'schemas_src_sha':src.get('sha',''),'tree':tree.get('tree',[]),'truncated':bool(tree.get('truncated',False))}
(out/'tree.json').write_text(json.dumps(meta, indent=2, sort_keys=True)+'\n')
print(f'optimade-schema-catalog updated: {out}/tree.json entries={len(meta["tree"])} ref={ref}')
PY
( cd "$OUT" && shasum -a 256 tree.json > SHA256SUMS )
