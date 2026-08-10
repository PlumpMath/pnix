#!/usr/bin/env bash
# DataCite Metadata Schema XSD snapshot.
# Uses official datacite/schema tag. No DOI records, API harvest payloads, title/abstract/person values, resource URLs, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${DATACITE_SCHEMA_DEST:-$ROOT/ingest/scholarly/datacite-metadata-schema}"
REF="${DATACITE_SCHEMA_REF:-4.7.3}"
KERNEL="${DATACITE_SCHEMA_KERNEL:-kernel-4.7}"
TMP="${TMPDIR:-/tmp}/pnix-datacite-schema-$$"
rm -rf "$TMP"
git -c advice.detachedHead=false clone --depth 1 --branch "$REF" https://github.com/datacite/schema.git "$TMP" >/dev/null 2>&1
python3 - "$TMP" "$DEST" "$REF" "$KERNEL" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys
src=pathlib.Path(sys.argv[1]); dest=pathlib.Path(sys.argv[2]); ref=sys.argv[3]; kernel=sys.argv[4]
root=src/'source/meta'/kernel
paths=[p.relative_to(src).as_posix() for p in sorted(root.rglob('*.xsd'))]
extra=[]
if (src/'README.md').exists(): extra.append('README.md')
records=[]
for path in extra+paths:
    raw=(src/path).read_bytes()
    rel=pathlib.Path('raw')/path
    out=dest/rel; out.parent.mkdir(parents=True,exist_ok=True); out.write_bytes(raw)
    role='readme_hash_only' if path.endswith('README.md') else 'xsd_schema'
    records.append({'source_path':path,'relative_path':str(rel),'url':f'https://github.com/datacite/schema/blob/{ref}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'DataCite Metadata Schema XSD metadata','ref':ref,'kernel':kernel,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/datacite/schema','https://support.datacite.org/docs/harvesting-datacite-doi-metadata','https://support.datacite.org/docs/datacite-data-file-use-policy'],'license':'CC0-1.0 for DataCite metadata; XSD schema structure only','scope':'official XSD schema structure only; no DOI records/title/abstract/person values/landing URLs/resource payloads/API harvest results/graph wiring','files':records,'xsd_file_count':len(paths)}
(dest/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded DataCite schema snapshot: ref={ref} kernel={kernel} xsd_files={len(paths)} -> {dest}')
PY
rm -rf "$TMP"
