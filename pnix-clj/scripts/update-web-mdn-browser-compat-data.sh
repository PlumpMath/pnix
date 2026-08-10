#!/usr/bin/env bash
# MDN browser-compat-data official GitHub snapshot.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${MDN_BCD_DEST:-$ROOT/ingest/web/mdn-browser-compat-data}"
REF="${MDN_BCD_REF:-main}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, shutil, sys, tarfile, tempfile, urllib.request
DEST=pathlib.Path(sys.argv[1]); REF=sys.argv[2]
URL=f'https://api.github.com/repos/mdn/browser-compat-data/tarball/{REF}'
UA='pnix-mdn-bcd-ingest/1.0 (structural compatibility metadata only; no docs prose)'
req=urllib.request.Request(URL,headers={'User-Agent':UA,'Accept':'application/vnd.github+json'})
raw=urllib.request.urlopen(req,timeout=120).read()
archive_sha=hashlib.sha256(raw).hexdigest()
if (DEST/'raw').exists(): shutil.rmtree(DEST/'raw')
(DEST/'raw').mkdir(parents=True,exist_ok=True)
keep_roots={'api','browsers','css','html','http','javascript','mathml','svg','webassembly','webextensions'}
files=[]
with tempfile.NamedTemporaryFile(suffix='.tar.gz') as tmp:
    tmp.write(raw); tmp.flush()
    with tarfile.open(tmp.name,'r:gz') as tf:
        for member in tf.getmembers():
            if not member.isfile(): continue
            parts=pathlib.PurePosixPath(member.name).parts
            if len(parts)<2: continue
            rel=pathlib.PurePosixPath(*parts[1:])
            if rel.name in {'LICENSE','package.json'} or rel.parts[0] in keep_roots:
                if rel.suffix.lower() not in {'.json',''} and rel.name!='LICENSE': continue
                f=tf.extractfile(member)
                if f is None: continue
                data=f.read()
                out=DEST/'raw'/rel
                out.parent.mkdir(parents=True,exist_ok=True)
                out.write_bytes(data)
                files.append({'relative_path':str(pathlib.PurePosixPath('raw')/rel),'source_path':str(rel),'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'MDN browser-compat-data','retrieved_at':dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),'ref':REF,'archive_url':URL,'archive_sha256':archive_sha,'license':'CC0-1.0','scope':'official machine-readable browser compatibility data only; no documentation prose, examples, telemetry/logs, runtime probing, advice, or graph/mirror wiring','files':files}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded MDN BCD: ref={REF} files={len(files)} archive_bytes={len(raw)} -> {DEST}')
PY
