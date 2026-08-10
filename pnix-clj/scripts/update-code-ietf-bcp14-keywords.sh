#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/ietf-bcp14-keywords"
mkdir -p "$OUT/raw"
# Do not download RFC bodies. Store only official URL reachability/status metadata.
python3 - "$OUT" <<'PY'
import datetime, json, subprocess, sys
from pathlib import Path
out=Path(sys.argv[1])
urls=[
  ("RFC2119", "https://www.rfc-editor.org/rfc/rfc2119"),
  ("RFC8174", "https://www.rfc-editor.org/rfc/rfc8174"),
  ("BCP14", "https://www.rfc-editor.org/info/bcp14"),
]
sources=[]
for ident,url in urls:
    try:
        cp=subprocess.run(["curl","-L","-I","--fail","--retry","2","--connect-timeout","15",url], text=True, capture_output=True, timeout=60)
        status="ok" if cp.returncode==0 else "unavailable"
        header_lines=[x.strip() for x in cp.stdout.splitlines() if x.lower().startswith(("http/","last-modified:","etag:","content-type:"))]
    except Exception as e:
        status="error"; header_lines=[str(e)]
    sources.append({"id":ident,"url":url,"status":status,"headers":header_lines[:8]})
receipt={
  "schema":"pnix.ingest.source_receipt.v1",
  "source_id":"ietf-bcp14-keywords",
  "source_name":"IETF BCP 14 requirement keywords",
  "retrieved_at_utc":datetime.datetime.utcnow().replace(microsecond=0).isoformat()+"Z",
  "license":"IETF Trust Legal Provisions / token-only metadata",
  "sources":sources,
  "scope":"official URL/status metadata plus manually enumerated BCP14 keyword tokens only",
  "excluded":["RFC body text","examples","prose explanations","derived excerpts","requirements engine execution","mirror/graph wiring"]
}
out.mkdir(parents=True, exist_ok=True)
(out/"source-receipt.json").write_text(json.dumps(receipt, indent=2, ensure_ascii=False)+"\n")
print(f"updated {out}: sources={len(sources)} token_table=manual-bcp14")
PY
