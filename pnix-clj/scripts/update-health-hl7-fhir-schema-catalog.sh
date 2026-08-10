#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${FHIR_SCHEMA_DEST:-$ROOT/ingest/health/hl7-fhir-schema-catalog}"
mkdir -p "$DEST/raw"
python3 - "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, urllib.request, zipfile, io, sys
DEST=pathlib.Path(sys.argv[1]); UA='pnix-fhir-schema-ingest/1.0 (JSON Schema metadata only; no patient data)'
SOURCES=[('R4','https://hl7.org/fhir/R4/fhir.schema.json.zip'),('R5','https://hl7.org/fhir/R5/fhir.schema.json.zip')]
files=[]
for version,url in SOURCES:
    req=urllib.request.Request(url,headers={'User-Agent':UA})
    raw=urllib.request.urlopen(req,timeout=120).read()
    zsha=hashlib.sha256(raw).hexdigest()
    zpath=DEST/'raw'/f'{version}-fhir.schema.json.zip'; zpath.parent.mkdir(parents=True,exist_ok=True); zpath.write_bytes(raw)
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        for name in z.namelist():
            if not name.endswith('.json'): continue
            data=z.read(name)
            rel=f'raw/{version}/{pathlib.PurePosixPath(name).name}'
            p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(data)
            files.append({'version':version,'relative_path':rel,'zip_url':url,'zip_sha256':zsha,'name':name,'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'HL7 FHIR JSON Schema catalog','retrieved_at':dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':[u for _,u in SOURCES]+['https://hl7.org/fhir/license.html'],'license':'HL7 FHIR specification terms / CC0-style public specification content','scope':'official JSON Schema files only; no patient/resource instances, examples, terminology expansion payloads, narrative/prose, medical advice, runtime exchange/validation, or graph/mirror wiring','files':files}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded HL7 FHIR schemas: files={len(files)} -> {DEST}')
PY
