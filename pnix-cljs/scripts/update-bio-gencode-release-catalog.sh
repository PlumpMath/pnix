#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/gencode-release-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, pathlib, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
urls=[('human','https://www.gencodegenes.org/human/releases.html'),('mouse','https://www.gencodegenes.org/mouse/releases.html')]
files=[]
for kind,url in urls:
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-gencode-release-catalog/1.0'})
    with urllib.request.urlopen(req,timeout=60) as r:
        data=r.read(); ctype=r.headers.get('content-type') or ''
    rel=f'raw/{kind}-releases.html'
    (out/rel).write_bytes(data)
    files.append({'kind':kind,'url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'GENCODE release history pages','retrieved_at':datetime.date.today().isoformat(),'policy':'release/link catalog metadata only; no GTF/GFF/FASTA payload downloads','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
