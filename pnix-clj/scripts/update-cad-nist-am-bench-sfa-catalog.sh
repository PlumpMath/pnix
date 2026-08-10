#!/usr/bin/env bash
# NIST AM-Bench + STEP File Analyzer catalog updater. Metadata only; no source/code/data payloads.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/cad/nist-am-bench-sfa-catalog"
mkdir -p "$DEST/pages" "$DEST/github"
python3 - "$DEST" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
DEST=pathlib.Path(sys.argv[1])
ua={'User-Agent':'pnix-ingest/1.0 (NIST AM/SFA catalog metadata only)','Accept':'application/vnd.github+json'}
pages=[
 ('ambench','https://www.nist.gov/ambench'),
 ('ambench-data-links','https://www.nist.gov/ambench/direct-am-bench-data-links-and-referencing-guidance'),
 ('ambench-dms','https://www.nist.gov/ambench/am-bench-data-management-systems'),
 ('sfa','https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer'),
]
repos=['usnistgov/SFA','usnistgov/ambench','usnistgov/AMB2022-template']
def fetch(url, accept=None):
    headers={'User-Agent':ua['User-Agent']}
    if accept: headers['Accept']=accept
    req=urllib.request.Request(url,headers=headers)
    with urllib.request.urlopen(req,timeout=90) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
page_receipts=[]
for slug,url in pages:
    raw,final,ctype=fetch(url)
    (DEST/'pages'/f'{slug}.html').write_bytes(raw)
    page_receipts.append({'slug':slug,'url':url,'final_url':final,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'content_type':ctype})
repo_receipts=[]
for repo in repos:
    raw,final,ctype=fetch(f'https://api.github.com/repos/{repo}', 'application/vnd.github+json')
    (DEST/'github'/(repo.replace('/','__')+'.repo.json')).write_bytes(raw)
    tags_raw,tags_final,tags_ctype=fetch(f'https://api.github.com/repos/{repo}/tags?per_page=20','application/vnd.github+json')
    (DEST/'github'/(repo.replace('/','__')+'.tags.json')).write_bytes(tags_raw)
    repo_receipts.append({'repo':repo,'repo_url':final,'repo_sha256':hashlib.sha256(raw).hexdigest(),'repo_size_bytes':len(raw),'repo_content_type':ctype,'tags_url':tags_final,'tags_sha256':hashlib.sha256(tags_raw).hexdigest(),'tags_size_bytes':len(tags_raw),'tags_content_type':tags_ctype})
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'NIST AM-Bench and STEP File Analyzer public project/repository catalog metadata',
 'version':'snapshot-2026-06-19',
 'pages':page_receipts,
 'repositories':repo_receipts,
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'NIST public metadata / public domain where applicable; repository code license not asserted by this ingest',
 'scope':'official NIST page hash metadata and GitHub repository metadata only; source/code/prose/data/CAD/STEP/toolpath/process payloads excluded'
}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated NIST AM/SFA catalog: pages={len(page_receipts)} repos={len(repo_receipts)}')
PY
