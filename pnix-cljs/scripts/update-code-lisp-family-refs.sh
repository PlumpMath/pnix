#!/usr/bin/env bash
set -euo pipefail

# SBCL / Racket / Chez Scheme 공식 repo에서 Lisp-family 구조 원천 다운로드.
# host=네트워크/파일 IO와 version pinning만 수행; graph/math 연결 없음.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/lisp-family"
mkdir -p "$OUT"

python3 - "$OUT" <<'PY'
import hashlib,json,pathlib,subprocess,urllib.request,sys
out=pathlib.Path(sys.argv[1])
sources={
 'sbcl': {'repo':'https://github.com/sbcl/sbcl','git':'https://github.com/sbcl/sbcl.git','branch':'master','license_id':'Public-Domain/FreeBSD-style','files':['COPYING','src/cold/common-lisp-exports.lisp-expr','src/cold/exports.lisp','src/code/reader.lisp']},
 'racket': {'repo':'https://github.com/racket/racket','git':'https://github.com/racket/racket.git','branch':'master','license_id':'MIT OR Apache-2.0 (fact rows only; runtime components excluded)','files':['LICENSE.txt','pkgs/racket-doc/scribblings/reference/syntax.scrbl','pkgs/racket-doc/scribblings/reference/reader.scrbl','racket/src/expander/namespace/core.rkt']},
 'chez': {'repo':'https://github.com/cisco/ChezScheme','git':'https://github.com/cisco/ChezScheme.git','branch':'main','license_id':'Apache-2.0','files':['LICENSE','s/primdata.ss','s/syntax.ss','s/base-lang.ss','csug/priminfo.ss']},
}
def ls_remote(url,branch):
    cp=subprocess.run(['git','ls-remote',url,f'refs/heads/{branch}'],text=True,capture_output=True,check=True)
    return cp.stdout.split()[0]
def fetch(url):
    with urllib.request.urlopen(url,timeout=30) as r: return r.read()
included=[]
for sid,cfg in sources.items():
    commit=ls_remote(cfg['git'],cfg['branch'])
    base=f"{cfg['repo'].replace('github.com','raw.githubusercontent.com')}/{commit}"
    sdir=out/sid; sdir.mkdir(parents=True,exist_ok=True)
    (sdir/'COMMIT').write_text(commit+'\n')
    files=[]
    for rel in cfg['files']:
        b=fetch(f'{base}/{rel}')
        target=sdir/rel; target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes(b)
        files.append({'path':rel,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})
    included.append({'source_id':sid,'repo_url':cfg['repo'],'commit_sha':commit,'default_branch':cfg['branch'],'license_id':cfg['license_id'],'files':files})
manifest={'schema':'pnix.ingest.source_manifest.v1','source_id':'lisp-family-refs','source_name':'Scheme/Racket/Common Lisp structural refs','retrieved_at':'2026-06-19','license_policy':'Permissive/fact-row extraction only. SBCL CL symbol exports, Racket syntax/reader form names, Chez primitive symbol rows. No source/prose/docstring bodies; no graph/math wiring.','included':included}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f"downloaded lisp-family refs -> {out}")
for s in included: print(f"  {s['source_id']} {s['commit_sha'][:12]} files={len(s['files'])}")
PY
