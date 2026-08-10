#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${GHA_WORKFLOW_SYNTAX_SRC:-$ROOT/ingest/devops/github-actions-workflow-syntax}"
URL="${GHA_WORKFLOW_SYNTAX_URL:-https://raw.githubusercontent.com/github/docs/main/content/actions/reference/workflows-and-actions/workflow-syntax.md}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$URL" <<'PY'
import datetime as dt, hashlib, json, urllib.request, sys
from pathlib import Path

dest=Path(sys.argv[1]); url=sys.argv[2]
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)
req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/github-actions-workflow-syntax'})
with urllib.request.urlopen(req, timeout=30) as r:
    data=r.read(); headers=dict(r.headers); final_url=r.geturl()
(raw/'workflow-syntax.md').write_bytes(data)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'GitHub Actions workflow syntax reference','license':'CC-BY-4.0','url':url,'final_url':final_url,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),'content_type':headers.get('Content-Type',''),'scope':'workflow YAML key/token metadata only; prose/examples/real workflow data/secrets/logs/artifacts/deployments/graph wiring excluded'}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f'updated GitHub Actions workflow syntax: bytes={len(data)} dest={dest}')
PY
