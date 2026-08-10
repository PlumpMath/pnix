#!/usr/bin/env bash
set -euo pipefail

# WebAssembly/spec 공식 interpreter 구조 원천 다운로드.
# 의미 변환 없음: host는 네트워크/파일 IO와 버전 pinning만 수행한다.
# 생성/적재:
#   scripts/gen-code-wasm-spec.sh
#   ./target/debug/pnixc-meta --morph-rules-build stdlib/lib/corpus/wasm-spec-store-plan.px

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/wasm"
REPO_API="https://api.github.com/repos/WebAssembly/spec/commits/main"
mkdir -p "$OUT/syntax" "$OUT/binary"

commit_sha="$(curl -fsSL "$REPO_API" | python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])')"
base="https://raw.githubusercontent.com/WebAssembly/spec/${commit_sha}"

files=(
  "interpreter/LICENSE"
  "interpreter/syntax/mnemonics.ml"
  "interpreter/syntax/types.ml"
  "interpreter/syntax/ast.ml"
  "interpreter/binary/encode.ml"
  "interpreter/binary/decode.ml"
)

for p in "${files[@]}"; do
  rel="${p#interpreter/}"
  mkdir -p "$OUT/$(dirname "$rel")"
  curl -fsSL "$base/$p" -o "$OUT/$rel"
done

python3 - "$OUT" "$commit_sha" <<'PY'
import hashlib, json, pathlib, sys
out = pathlib.Path(sys.argv[1])
commit = sys.argv[2]
paths = [
  "LICENSE",
  "syntax/mnemonics.ml",
  "syntax/types.ml",
  "syntax/ast.ml",
  "binary/encode.ml",
  "binary/decode.ml",
]
files = []
for rel in paths:
    b = (out / rel).read_bytes()
    files.append({"path": rel, "sha256": hashlib.sha256(b).hexdigest(), "bytes": len(b)})
manifest = {
    "schema": "pnix.ingest.source_manifest.v1",
    "source_id": "wasm-spec",
    "source_name": "WebAssembly specification interpreter",
    "repo": "https://github.com/WebAssembly/spec",
    "commit_sha": commit,
    "retrieved_at": "2026-06-19",
    "license_id": "Apache-2.0",
    "files": files,
}
(out / "source-manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"downloaded wasm-spec {commit} -> {out}")
for f in files:
    print(f"  {f['sha256']}  {f['path']}  {f['bytes']} bytes")
PY
