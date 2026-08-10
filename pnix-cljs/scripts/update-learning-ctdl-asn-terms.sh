#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/learning/ctdl-asn-terms"
URL="${CTDL_ASN_TERMS_URL:-https://credreg.net/ctdlasn/terms}"
mkdir -p "$OUT/raw"
python3 - "$OUT" "$URL" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); url=sys.argv[2]
req=urllib.request.Request(url,headers={'User-Agent':'pnix-ctdl-asn-ingest/1.0','Accept':'text/turtle'})
with urllib.request.urlopen(req,timeout=60) as r:
    data=r.read(); ctype=r.headers.get('content-type')
raw=out/'raw'/'ctdl-asn-terms.ttl'
raw.write_bytes(data)
manifest={'schema':'pnix.source_manifest.v1','source':'CTDL-ASN schema terms Turtle','url':url,'retrieved_at':datetime.date.today().isoformat(),'content_type':ctype,'policy':'Turtle schema terms only; definitions/comments/examples/registry payloads excluded by generator','files':[{'path':'raw/ctdl-asn-terms.ttl','url':url,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype}]}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'bytes':len(data),'content_type':ctype},ensure_ascii=False))
PY
