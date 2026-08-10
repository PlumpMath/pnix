#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/scim-schema-tokens"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, json, subprocess, sys
from pathlib import Path
out=Path(sys.argv[1])
urls=[("RFC7643","https://www.rfc-editor.org/rfc/rfc7643"),("RFC7644","https://www.rfc-editor.org/rfc/rfc7644")]
sources=[]
for ident,url in urls:
    cp=subprocess.run(["curl","-L","-I","--fail","--retry","2","--connect-timeout","15",url], text=True, capture_output=True, timeout=60)
    headers=[x.strip() for x in cp.stdout.splitlines() if x.lower().startswith(("http/","last-modified:","etag:","content-type:"))]
    sources.append({"id":ident,"url":url,"status":"ok" if cp.returncode==0 else "unavailable","headers":headers[:8]})
receipt={"schema":"pnix.ingest.source_receipt.v1","source_id":"scim-schema-tokens","source_name":"SCIM RFC 7643/7644 schema and protocol token metadata","retrieved_at_utc":datetime.datetime.utcnow().replace(microsecond=0).isoformat()+"Z","license":"IETF Trust Legal Provisions / token-only metadata","sources":sources,"scope":"official URL/status metadata plus manually enumerated SCIM schema/protocol tokens only","excluded":["RFC body text","examples","prose explanations","live IAM exports","user/group records","credentials","authorization decisions","provisioning execution","mirror/graph wiring"]}
out.mkdir(parents=True, exist_ok=True)
(out/"source-receipt.json").write_text(json.dumps(receipt, indent=2, ensure_ascii=False)+"\n")
print(f"updated {out}: sources={len(sources)} token_table=manual-scim")
PY
