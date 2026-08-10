#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/arrow-parquet-avro-specification"
ARROW_REF="${ARROW_SPEC_REF:-main}"
PARQUET_REF="${PARQUET_FORMAT_REF:-master}"
AVRO_REF="${AVRO_SPEC_REF:-main}"
mkdir -p "$DST/arrow/format" "$DST/parquet/src/main/thrift" "$DST/avro/share/idl_grammar/org/apache/avro/idl" "$DST/avro/share/schemas/org/apache/avro/data" "$DST/avro/share/schemas/org/apache/avro/ipc" "$DST/avro/share/schemas/org/apache/avro/mapred/tether"
for f in File.fbs Message.fbs Schema.fbs SparseTensor.fbs Tensor.fbs; do
  curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/arrow/format/$f" "https://raw.githubusercontent.com/apache/arrow/$ARROW_REF/format/$f"
done
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/parquet/src/main/thrift/parquet.thrift" "https://raw.githubusercontent.com/apache/parquet-format/$PARQUET_REF/src/main/thrift/parquet.thrift"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/avro/share/idl_grammar/org/apache/avro/idl/Idl.g4" "https://raw.githubusercontent.com/apache/avro/$AVRO_REF/share/idl_grammar/org/apache/avro/idl/Idl.g4"
for f in \
  share/schemas/org/apache/avro/data/Json.avsc \
  share/schemas/org/apache/avro/ipc/HandshakeRequest.avsc \
  share/schemas/org/apache/avro/ipc/HandshakeResponse.avsc \
  share/schemas/org/apache/avro/mapred/tether/InputProtocol.avpr \
  share/schemas/org/apache/avro/mapred/tether/OutputProtocol.avpr
 do
  mkdir -p "$DST/avro/$(dirname "$f")"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/avro/$f" "https://raw.githubusercontent.com/apache/avro/$AVRO_REF/$f"
done
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
python3 - "$DST" "$ARROW_REF" "$PARQUET_REF" "$AVRO_REF" <<'PY'
import pathlib, sys, json, hashlib, datetime
root=pathlib.Path(sys.argv[1]); refs={'arrow':sys.argv[2],'parquet':sys.argv[3],'avro':sys.argv[4]}
files=[]
for p in sorted(root.rglob('*')):
    if p.is_file() and p.name!='source-manifest.json':
        b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'arrow-parquet-avro-specification','source_name':'Apache Arrow / Parquet / Avro structural schemas','license_id':'Apache-2.0','refs':refs,'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z','files':files,'policy':'Official schema/IDL structure only. Exclude data files, examples/tests, customer schemas, schema registry subjects, docs prose, execution, graph wiring.'},indent=2),encoding='utf-8')
print(f'updated {root}/source-manifest.json files={len(files)} refs={refs}')
PY
