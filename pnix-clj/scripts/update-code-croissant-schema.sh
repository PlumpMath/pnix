#!/usr/bin/env bash
# MLCommons Croissant implementation schema snapshot.
# Uses official git tag. No spec prose/docs/examples/dataset payloads/credentials/execution/graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CROISSANT_SCHEMA_DEST:-$ROOT/ingest/code/croissant-schema}"
REF="${CROISSANT_SCHEMA_REF:-v1.1.0}"
TMP="${TMPDIR:-/tmp}/pnix-croissant-schema-$$"
rm -rf "$TMP"
git -c advice.detachedHead=false clone --depth 1 --branch "$REF" https://github.com/mlcommons/croissant.git "$TMP" >/dev/null 2>&1
python3 - "$TMP" "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, shutil, sys
src=pathlib.Path(sys.argv[1]); dest=pathlib.Path(sys.argv[2]); ref=sys.argv[3]
paths=[
 'LICENSE.md',
 'python/mlcroissant/mlcroissant/_src/core/constants.py',
 'python/mlcroissant/mlcroissant/_src/core/context.py',
 'python/mlcroissant/mlcroissant/_src/core/data_types.py',
 'python/mlcroissant/mlcroissant/_src/core/dataclasses.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/base_node.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/graph.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/creative_work.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/field.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/file_object.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/file_set.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/metadata.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/organization.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/person.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/record_set.py',
 'python/mlcroissant/mlcroissant/_src/structure_graph/nodes/source.py',
]
records=[]
for path in paths:
    raw=(src/path).read_bytes()
    rel=pathlib.Path('raw')/path
    out=dest/rel; out.parent.mkdir(parents=True,exist_ok=True); out.write_bytes(raw)
    role='license' if path=='LICENSE.md' else 'implementation_schema_source'
    records.append({'source_path':path,'relative_path':str(rel),'url':f'https://github.com/mlcommons/croissant/blob/{ref}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'MLCommons Croissant mlcroissant implementation schema metadata','ref':ref,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/mlcommons/croissant','https://github.com/mlcommons/croissant/tree/'+ref],'license':'Apache-2.0 for implementation; CC-BY-ND spec prose excluded','scope':'implementation schema structure only; no CC-BY-ND spec prose/docs/examples/dataset metadata/payloads/credentials/execution/graph wiring','files':records,'selected_file_count':len(paths)}
(dest/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded Croissant implementation schema snapshot: ref={ref} files={len(paths)-1} -> {dest}')
PY
rm -rf "$TMP"
