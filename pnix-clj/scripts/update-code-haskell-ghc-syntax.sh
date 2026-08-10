#!/usr/bin/env bash
set -euo pipefail

# GHC 공식 repo에서 Haskell syntax/parser 구조 원천 다운로드.
# host=네트워크/파일 IO와 version pinning만 수행; graph/math 연결 없음.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/haskell/ghc"
mkdir -p "$OUT"
commit_sha="$(git ls-remote https://github.com/ghc/ghc.git refs/heads/master | awk '{print $1}')"
base="https://raw.githubusercontent.com/ghc/ghc/${commit_sha}"
files=(
  "LICENSE"
  "compiler/GHC/Parser.y"
  "compiler/GHC/Parser/Lexer.x"
  "compiler/Language/Haskell/Syntax/Expr.hs"
  "compiler/Language/Haskell/Syntax/Pat.hs"
  "compiler/Language/Haskell/Syntax/Type.hs"
  "compiler/Language/Haskell/Syntax/Decls.hs"
  "compiler/Language/Haskell/Syntax/Binds.hs"
  "compiler/Language/Haskell/Syntax/Basic.hs"
  "libraries/ghc-internal/src/GHC/Internal/LanguageExtensions.hs"
)
for p in "${files[@]}"; do
  mkdir -p "$OUT/$(dirname "$p")"
  curl -fsSL "$base/$p" -o "$OUT/$p"
done
echo "$commit_sha" > "$OUT/COMMIT"
python3 - "$OUT" "$commit_sha" <<'PY'
import hashlib,json,pathlib,sys
out=pathlib.Path(sys.argv[1]); commit=sys.argv[2]
files=[]
for p in sorted(x for x in out.rglob('*') if x.is_file() and x.name not in {'COMMIT','source-manifest.json'}):
    b=p.read_bytes(); rel=str(p.relative_to(out))
    files.append({'path':rel,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})
manifest={'schema':'pnix.ingest.source_manifest.v1','source_id':'ghc-syntax','source_name':'GHC exposed Haskell syntax/parser structures','repo':'https://github.com/ghc/ghc','commit_sha':commit,'retrieved_at':'2026-06-19','license_id':'GHC BSD-3-Clause-style','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f"downloaded ghc-syntax {commit} -> {out}")
for f in files: print(f"  {f['sha256']}  {f['path']}  {f['bytes']} bytes")
PY
