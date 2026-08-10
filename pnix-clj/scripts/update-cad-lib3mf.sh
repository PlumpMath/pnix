#!/usr/bin/env bash
# 3MF Consortium lib3mf schema snapshot.
# Uses official lib3mf tag. No model payloads, geometry values, print/process params, generated bindings, execution, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${LIB3MF_DEST:-$ROOT/ingest/cad/lib3mf}"
REF="${LIB3MF_REF:-v2.5.0}"
TMP="${TMPDIR:-/tmp}/pnix-lib3mf-$$"
rm -rf "$TMP"
git -c advice.detachedHead=false clone --depth 1 --branch "$REF" https://github.com/3MFConsortium/lib3mf.git "$TMP" >/dev/null 2>&1
python3 - "$TMP" "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys
src=pathlib.Path(sys.argv[1]); dest=pathlib.Path(sys.argv[2]); ref=sys.argv[3]
paths=['LICENSE','AutomaticComponentToolkit/lib3mf.xml']
paths += [p.relative_to(src).as_posix() for p in sorted((src/'Tests/TestFiles/Schema').glob('*.xsd'))]
records=[]
for path in paths:
    raw=(src/path).read_bytes()
    rel=pathlib.Path('raw')/path
    out=dest/rel; out.parent.mkdir(parents=True,exist_ok=True); out.write_bytes(raw)
    role='license' if path=='LICENSE' else ('interface_xml' if path.endswith('.xml') else 'xsd_schema')
    records.append({'source_path':path,'relative_path':str(rel),'url':f'https://github.com/3MFConsortium/lib3mf/blob/{ref}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'3MF Consortium lib3mf schema metadata','ref':ref,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/3MFConsortium/lib3mf','https://github.com/3MFConsortium/lib3mf/tree/'+ref],'license':'BSD-2-Clause','scope':'interface XML and XSD schema structure only; no model payloads/geometry values/printer parameters/generated bindings/prose docs/execution/graph wiring','files':records,'selected_file_count':len(paths)-1}
(dest/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded lib3mf schema snapshot: ref={ref} files={len(paths)-1} -> {dest}')
PY
rm -rf "$TMP"
