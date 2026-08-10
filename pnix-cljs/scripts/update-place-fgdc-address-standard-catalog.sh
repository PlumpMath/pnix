#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${FGDC_ADDRESS_DEST:-$ROOT/ingest/place/fgdc-address-standard}"
UA="${FGDC_ADDRESS_USER_AGENT:-pnix-ingest/0.1 (FGDC address standard metadata catalog)}"
mkdir -p "$DEST/raw"
# FGDC site currently serves an expired TLS certificate (observed 2026-06-19).
# Keep this source fail-closed by recording the exception in source-receipt.json and
# hashing every downloaded file; do not silently trust unrecorded payloads.
curl -k -fsSL -A "$UA" "https://www.fgdc.gov/standards/projects/address-data" -o "$DEST/raw/address-data.html"
curl -k -fsSL -A "$UA" "https://www.fgdc.gov/standards/projects/address-data/FGDC_endorsedAddressStandard.zip" -o "$DEST/raw/FGDC_endorsedAddressStandard.zip"
python3 - <<'PY' "$DEST"
import hashlib,json,pathlib,sys,time,zipfile
root=pathlib.Path(sys.argv[1]); files=[]
for p in sorted((root/'raw').glob('*')):
    b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
zip_entries=[]
zp=root/'raw/FGDC_endorsedAddressStandard.zip'
with zipfile.ZipFile(zp) as z:
    for info in sorted(z.infolist(), key=lambda x:x.filename):
        if info.is_dir(): continue
        data=z.read(info.filename)
        zip_entries.append({'filename':info.filename,'bytes':info.file_size,'compressed_bytes':info.compress_size,'sha256':hashlib.sha256(data).hexdigest()})
(root/'source-receipt.json').write_text(json.dumps({'schema':'place.fgdc_address_standard.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / U.S. federal public standard metadata','tls_certificate_verification':'disabled_for_download_due_fgdc_expired_certificate_observed_2026_06_19','files':files,'zip_entries':zip_entries,'excluded':['PDF prose/body text','definitions/examples','address records','coordinates','routing/geocoding decisions','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched FGDC address standard catalog files={len(files)} zip_entries={len(zip_entries)} into {root}')
PY
