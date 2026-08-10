#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/governance/service-guidance-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, os, pathlib, re, urllib.parse, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
ua='pnix-service-guidance-catalog/1.0'
html_sources=[
 ('nasa_se','NASA Systems Engineering Handbook page','US-PD','https://www.nasa.gov/reference/systems-engineering-handbook/'),
 ('nasa_swehb','NASA Software Engineering Handbook page','US-PD','https://swehb.nasa.gov/'),
 ('usds_playbook','USDS Digital Services Playbook','CC0-style US public guidance','https://playbook.cio.gov/'),
 ('govuk_service_manual','GOV.UK Service Manual','OGL-3.0 attribution required','https://www.gov.uk/service-manual')
]
files=[]
def fetch(url):
    req=urllib.request.Request(url,headers={'User-Agent':ua})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read(), r.headers.get('content-type') or ''
for key,label,license_id,url in html_sources:
    data,ctype=fetch(url)
    rel=f'raw/{key}.html'
    (out/rel).write_bytes(data)
    files.append({'kind':'html_page','source_id':key,'label':label,'license':license_id,'url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
# NASA SE PDF is large prose; record HEAD metadata only, do not download.
pdf_url=os.environ.get('NASA_SE_PDF_URL','https://www.nasa.gov/wp-content/uploads/2018/09/nasa_systems_engineering_handbook_0.pdf')
try:
    req=urllib.request.Request(pdf_url,headers={'User-Agent':ua},method='HEAD')
    with urllib.request.urlopen(req,timeout=30) as r:
        files.append({'kind':'pdf_head_only','source_id':'nasa_se_pdf','label':'NASA Systems Engineering Handbook PDF (HEAD metadata only)','license':'US-PD','url':pdf_url,'path':'','bytes':int(r.headers.get('content-length') or 0),'sha256':'','content_type':r.headers.get('content-type') or ''})
except Exception as e:
    files.append({'kind':'blocked_head','source_id':'nasa_se_pdf','label':'NASA Systems Engineering Handbook PDF HEAD failed','license':'US-PD','url':pdf_url,'path':'','bytes':0,'sha256':'','content_type':'','error':type(e).__name__+': '+str(e)[:160]})
# 18F Guides: GitHub repo/tree metadata only, no file bodies.
api='https://api.github.com/repos/18F/guides'
req=urllib.request.Request(api,headers={'User-Agent':ua,'Accept':'application/vnd.github+json'})
with urllib.request.urlopen(req,timeout=60) as r:
    repo=json.loads(r.read().decode()); ctype=r.headers.get('content-type') or ''
rel='raw/18f-guides-repo.json'; data=json.dumps(repo,ensure_ascii=False,indent=2).encode()
(out/rel).write_bytes(data)
files.append({'kind':'github_repo_json','source_id':'18f_guides','label':'18F Guides GitHub repository metadata','license':'CC0-style US public guidance','url':api,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
branch=repo.get('default_branch') or 'main'
tree_url=f'https://api.github.com/repos/18F/guides/git/trees/{urllib.parse.quote(branch)}?recursive=1'
req=urllib.request.Request(tree_url,headers={'User-Agent':ua,'Accept':'application/vnd.github+json'})
with urllib.request.urlopen(req,timeout=60) as r:
    tree=json.loads(r.read().decode()); ctype=r.headers.get('content-type') or ''
rel='raw/18f-guides-tree.json'; data=json.dumps(tree,ensure_ascii=False,indent=2).encode()
(out/rel).write_bytes(data)
files.append({'kind':'github_tree_json','source_id':'18f_guides','label':'18F Guides Git tree metadata','license':'CC0-style US public guidance','url':tree_url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'NASA/USDS/18F/GOV.UK guidance catalog sources','retrieved_at':datetime.date.today().isoformat(),'policy':'page/link/heading/path metadata only; no prose bodies/PDF bodies/examples/templates/source-code/runtime data','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files)},ensure_ascii=False))
PY
