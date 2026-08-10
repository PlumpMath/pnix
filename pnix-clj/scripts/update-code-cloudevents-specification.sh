#!/usr/bin/env bash
# CloudEvents official release specification snapshot.
# Downloads selected official spec/binding/format files only. No payloads, endpoints, credentials, runtime invocation, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CLOUDEVENTS_SPEC_DEST:-$ROOT/ingest/code/cloudevents-specification}"
REF="${CLOUDEVENTS_SPEC_REF:-ce@v1.0.2}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
REF_URL=urllib.parse.quote(REF, safe='')
RAW=f'https://raw.githubusercontent.com/cloudevents/spec/{REF_URL}'
FILES=[
    'cloudevents/spec.md',
    'cloudevents/bindings/http-protocol-binding.md',
    'cloudevents/bindings/amqp-protocol-binding.md',
    'cloudevents/bindings/kafka-protocol-binding.md',
    'cloudevents/bindings/mqtt-protocol-binding.md',
    'cloudevents/bindings/nats-protocol-binding.md',
    'cloudevents/bindings/websockets-protocol-binding.md',
    'cloudevents/formats/json-format.md',
    'cloudevents/formats/protobuf-format.md',
    'cloudevents/formats/avro-format.md',
    'cloudevents/formats/cloudevents.json',
    'cloudevents/formats/cloudevents.avsc',
    'cloudevents/formats/cloudevents.proto',
]
def get_bytes(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-cloudevents-spec-ingest'})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read()
records=[]
for path in FILES:
    raw=get_bytes(f'{RAW}/{path}')
    rel=pathlib.Path('raw')/path
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    role='format_schema' if path.endswith(('.json','.avsc','.proto')) else 'spec_doc'
    records.append({'source_path':path,'relative_path':str(rel),'url':f'{RAW}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
lic=get_bytes(f'{RAW}/LICENSE')
(DEST/'LICENSE').write_bytes(lic)
records.append({'source_path':'LICENSE','relative_path':'LICENSE','url':f'{RAW}/LICENSE','sha256':hashlib.sha256(lic).hexdigest(),'size_bytes':len(lic),'role':'license'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'CloudEvents official specification documents and format schemas','ref':REF,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/cloudevents/spec','https://github.com/cloudevents/spec/releases'],'license':'Apache-2.0','scope':'official release core spec/binding/format structure only; no full prose bodies, SDK/primer/webhook narrative, subscriptions runtime, payload logs, live endpoints, credentials, invocation, or graph wiring','files':records,'selected_file_count':len(FILES)}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded CloudEvents spec snapshot: ref={REF} files={len(FILES)} -> {DEST}')
PY
