#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NIST_ELECTION_CDF_DEST:-$ROOT/ingest/election/nist-results-cdf}"
UA="${NIST_ELECTION_CDF_USER_AGENT:-pnix-ingest/0.1 (NIST Election Results CDF schema metadata)}"
mkdir -p "$DEST/raw"
python3 - <<'PY' "$DEST" "$UA"
import hashlib,json,os,pathlib,sys,time,urllib.request,zipfile,io
root=pathlib.Path(sys.argv[1]); ua=sys.argv[2]
headers={'User-Agent':ua,'Accept':'application/vnd.github+json'}
def fetch(url):
    req=urllib.request.Request(url,headers=headers)
    with urllib.request.urlopen(req,timeout=60) as r: return r.read()
repo=json.loads(fetch('https://api.github.com/repos/usnistgov/ElectionResultsReporting').decode())
ref=os.environ.get('NIST_ELECTION_CDF_REF','v2.0.3')
zip_url=f'https://github.com/usnistgov/ElectionResultsReporting/archive/refs/tags/{ref}.zip'
zip_bytes=fetch(zip_url)
(root/'raw/repo.json').write_text(json.dumps(repo,ensure_ascii=False,indent=2)+'\n')
(root/'raw/repo.zip').write_bytes(zip_bytes)
files=[{'path':'raw/repo.json','url':'https://api.github.com/repos/usnistgov/ElectionResultsReporting','sha256':hashlib.sha256((root/'raw/repo.json').read_bytes()).hexdigest(),'bytes':(root/'raw/repo.json').stat().st_size},{'path':'raw/repo.zip','url':zip_url,'sha256':hashlib.sha256(zip_bytes).hexdigest(),'bytes':len(zip_bytes)}]
extracted=[]
with zipfile.ZipFile(io.BytesIO(zip_bytes)) as z:
    for info in sorted(z.infolist(), key=lambda x:x.filename):
        if info.is_dir(): continue
        rel='/'.join(info.filename.split('/')[1:])
        lower=rel.lower()
        if not (lower.endswith('.xsd') or lower.endswith('.json') or lower.endswith('.jsonschema')): continue
        data=z.read(info.filename)
        out=root/'schema-files'/rel
        out.parent.mkdir(parents=True,exist_ok=True); out.write_bytes(data)
        extracted.append({'path':str(out.relative_to(root)),'repo_path':rel,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
(root/'source-receipt.json').write_text(json.dumps({'schema':'election.nist_results_cdf.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / NIST public schema metadata','repo':{'full_name':repo.get('full_name'),'ref':ref,'html_url':repo.get('html_url'),'pushed_at':repo.get('pushed_at')},'files':files,'schema_files':extracted,'excluded':['election result payloads','cast vote records','voter records','test data values','PDF/Word prose','examples','tabulation/audit/legal guidance','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched NIST ElectionResultsReporting ref={ref} schema_files={len(extracted)} into {root}')
PY
