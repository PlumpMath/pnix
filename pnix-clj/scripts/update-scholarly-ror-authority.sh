#!/usr/bin/env bash
# ROR official organization authority snapshot.
# Uses official ror-records git tag. No coordinates/address lines/domains/URL payloads/email/IP/live API harvest/graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ROR_DEST:-$ROOT/ingest/scholarly/ror-authority}"
REF="${ROR_REF:-v2.8}"
VERSION_DIR="${ROR_VERSION_DIR:-v2.8}"
LIMIT="${ROR_LIMIT:-250}"
TMP="${TMPDIR:-/tmp}/pnix-ror-records-$$"
rm -rf "$TMP"
git -c advice.detachedHead=false clone --depth 1 --branch "$REF" https://github.com/ror-community/ror-records.git "$TMP" >/dev/null 2>&1
rm -rf "$DEST"
python3 - "$TMP" "$DEST" "$REF" "$VERSION_DIR" "$LIMIT" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys
src=pathlib.Path(sys.argv[1]); dest=pathlib.Path(sys.argv[2]); ref=sys.argv[3]; version_dir=sys.argv[4]; limit=int(sys.argv[5])
root=src/version_dir
records=[]
paths=sorted(root.glob('*.json'))
for path in paths[:limit]:
    raw=path.read_bytes()
    rel=pathlib.Path('raw')/version_dir/path.name
    out=dest/rel; out.parent.mkdir(parents=True,exist_ok=True); out.write_bytes(raw)
    records.append({'source_path':f'{version_dir}/{path.name}','relative_path':str(rel),'url':f'https://github.com/ror-community/ror-records/blob/{ref}/{version_dir}/{path.name}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'ror_record_json'})
lic=(src/'LICENSE').read_bytes()
(dest/'LICENSE').write_bytes(lic)
records.append({'source_path':'LICENSE','relative_path':'LICENSE','url':f'https://github.com/ror-community/ror-records/blob/{ref}/LICENSE','sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
readme=(src/'README.md').read_bytes()
(dest/'README.md').write_bytes(readme)
records.append({'source_path':'README.md','relative_path':'README.md','url':f'https://github.com/ror-community/ror-records/blob/{ref}/README.md','sha256':hashlib.sha256(readme).hexdigest(),'size_bytes':len(readme),'role':'readme_hash_only'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'ROR official organization authority records','ref':ref,'version_dir':version_dir,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/ror-community/ror-records','https://ror.org/'],'license':'MIT repository license / open ROR authority data','scope':'bounded organization authority rows only; no coordinates/address lines/domains/URL payloads/email/IP/live API harvest/web page bodies/graph wiring','files':records,'record_file_count_total_in_version_dir':len(paths),'record_file_count_stored':min(len(paths),limit)}
(dest/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded ROR authority snapshot: ref={ref} version_dir={version_dir} stored={min(len(paths),limit)}/{len(paths)} -> {dest}')
PY
rm -rf "$TMP"
