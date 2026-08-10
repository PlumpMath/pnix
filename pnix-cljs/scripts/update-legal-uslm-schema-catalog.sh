#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/legal/uslm-schema-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, pathlib, urllib.parse, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
ua='pnix-uslm-schema-catalog/1.0'
repo_url='https://api.github.com/repos/usgpo/uslm'
req=urllib.request.Request(repo_url,headers={'User-Agent':ua,'Accept':'application/vnd.github+json'})
with urllib.request.urlopen(req,timeout=60) as r:
    repo=json.loads(r.read().decode()); ctype=r.headers.get('content-type') or ''
rel='raw/repo.json'; data=json.dumps(repo,ensure_ascii=False,indent=2).encode()
(out/rel).write_bytes(data)
files=[{'kind':'github_repo_json','url':repo_url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype}]
branch=repo.get('default_branch') or 'main'
tree_url=f'https://api.github.com/repos/usgpo/uslm/git/trees/{urllib.parse.quote(branch)}?recursive=1'
req=urllib.request.Request(tree_url,headers={'User-Agent':ua,'Accept':'application/vnd.github+json'})
with urllib.request.urlopen(req,timeout=60) as r:
    tree=json.loads(r.read().decode()); ctype=r.headers.get('content-type') or ''
rel='raw/tree.json'; data=json.dumps(tree,ensure_ascii=False,indent=2).encode()
(out/rel).write_bytes(data)
files.append({'kind':'github_tree_json','url':tree_url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'USGPO USLM GitHub repository metadata','retrieved_at':datetime.date.today().isoformat(),'policy':'repo/tree path metadata only; no US Code text, schema bodies, docs, samples or legal advice','files':files,'default_branch':branch,'repo_html_url':repo.get('html_url','')}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'branch':branch,'files':len(files)},ensure_ascii=False))
PY
