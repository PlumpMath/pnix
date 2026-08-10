#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REF="${TEKTON_PIPELINE_REF:-v1.13.1}"
DEST="${TEKTON_PIPELINE_API_SRC:-$ROOT/ingest/devops/tekton-pipeline-api}"
RAW="$DEST/raw"
mkdir -p "$RAW"
files=(300-pipeline.yaml 300-task.yaml 300-pipelinerun.yaml 300-taskrun.yaml)
base="https://raw.githubusercontent.com/tektoncd/pipeline/$REF/config/300-crds"
for f in "${files[@]}"; do
  url="$base/$f"
  echo "download $url"
  curl -fsSL "$url" -o "$RAW/$f"
done
python3 - "$DEST" "$REF" "$base" "${files[@]}" <<'PY'
import hashlib, json, os, sys, time
out, ref, base, *files = sys.argv[1:]
raw = os.path.join(out, 'raw')
items = []
for name in files:
    p = os.path.join(raw, name)
    b = open(p, 'rb').read()
    items.append({
        'file': name,
        'url': f'{base}/{name}',
        'sha256': hashlib.sha256(b).hexdigest(),
        'bytes': len(b),
    })
receipt = {
    'schema': 'pnix.ingest.source_receipt.v1',
    'source': 'Tekton Pipeline API CRDs',
    'source_id': 'tekton-pipeline-api',
    'source_ref': ref,
    'license': 'Apache-2.0',
    'retrieved_at_epoch': int(time.time()),
    'retrieved_at_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
    'files': items,
    'excluded': ['actual CR instances', 'secrets', 'logs', 'artifacts', 'execution/deployment behavior', 'prose/examples', 'mirror/graph wiring'],
}
os.makedirs(out, exist_ok=True)
with open(os.path.join(out, 'source-receipt.json'), 'w', encoding='utf-8') as f:
    json.dump(receipt, f, ensure_ascii=False, indent=2, sort_keys=True)
    f.write('\n')
print(f"wrote {os.path.join(out, 'source-receipt.json')}: files={len(items)} bytes={sum(x['bytes'] for x in items)}")
PY
