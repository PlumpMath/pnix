#!/usr/bin/env bash
set -euo pipefail

# Clojure/ClojureScript 공식 repo 구조 원천 다운로드.
# host=네트워크/파일 IO와 version pinning만 수행; graph/math 연결 없음.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/clojure"
mkdir -p "$OUT/clojure" "$OUT/clojurescript"

python3 - "$OUT" <<'PY'
import hashlib, json, pathlib, subprocess, sys, urllib.error, urllib.request
out=pathlib.Path(sys.argv[1])
sources={
  'clojure': {
    'repo_url':'https://github.com/clojure/clojure',
    'git_url':'https://github.com/clojure/clojure.git',
    'branch':'master',
    'license_id':'EPL-1.0',
    'files':['epl-v10.html','src/clj/clojure/core.clj','src/clj/clojure/repl.clj','src/jvm/clojure/lang/LispReader.java','src/jvm/clojure/lang/Compiler.java'],
  },
  'clojurescript': {
    'repo_url':'https://github.com/clojure/clojurescript',
    'git_url':'https://github.com/clojure/clojurescript.git',
    'branch':'master',
    'license_id':'EPL-1.0',
    'files':['epl-v10.html','src/main/cljs/cljs/core.cljs','src/main/clojure/cljs/analyzer.cljc'],
  },
}

def ls_remote(url, branch):
    cp=subprocess.run(['git','ls-remote',url,f'refs/heads/{branch}'], text=True, capture_output=True, check=True)
    return cp.stdout.split()[0]

def fetch(url):
    with urllib.request.urlopen(url, timeout=30) as r: return r.read()

included=[]
for sid,cfg in sources.items():
    commit=ls_remote(cfg['git_url'], cfg['branch'])
    base=f"{cfg['repo_url'].replace('github.com','raw.githubusercontent.com')}/{commit}"
    sdir=out/sid; sdir.mkdir(parents=True, exist_ok=True)
    (sdir/'COMMIT').write_text(commit+'\n')
    files=[]
    for rel in cfg['files']:
        b=fetch(f'{base}/{rel}')
        target=sdir/rel; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(b)
        files.append({'path':rel,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})
    included.append({'source_id':sid,'repo_url':cfg['repo_url'],'commit_sha':commit,'default_branch':cfg['branch'],'license_id':cfg['license_id'],'files':files})
manifest={'schema':'pnix.ingest.source_manifest.v1','source_id':'clojure-refs','source_name':'Clojure/ClojureScript language reference structures','retrieved_at':'2026-06-19','license_policy':'EPL-1.0 sources; generated redb payload contains fact rows only (symbols/categories/reader dispatch), no source code/prose/docstrings.','included':included}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n', encoding='utf-8')
print(f"downloaded clojure refs -> {out}")
for s in included:
    print(f"  {s['source_id']} {s['commit_sha'][:12]} files={len(s['files'])}")
PY
