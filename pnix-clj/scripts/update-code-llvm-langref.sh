#!/usr/bin/env bash
set -euo pipefail

# LLVM LangRef 공식 원천 다운로드. host=네트워크/파일 IO와 version pinning만 수행.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/llvm"
mkdir -p "$OUT"
commit_sha="$(curl -fsSL https://api.github.com/repos/llvm/llvm-project/commits/main | python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])')"
base="https://raw.githubusercontent.com/llvm/llvm-project/${commit_sha}"
curl -fsSL "$base/LICENSE.TXT" -o "$OUT/LICENSE.TXT"
curl -fsSL "$base/llvm/docs/LangRef.rst" -o "$OUT/LangRef.rst"
python3 - "$OUT" "$commit_sha" <<'PY'
import hashlib, json, pathlib, sys
out = pathlib.Path(sys.argv[1]); commit = sys.argv[2]
files=[]
for rel in ["LICENSE.TXT", "LangRef.rst"]:
    b=(out/rel).read_bytes()
    files.append({"path": rel, "sha256": hashlib.sha256(b).hexdigest(), "bytes": len(b), "lines": len(b.decode('utf-8','replace').splitlines())})
manifest={"schema":"pnix.ingest.source_manifest.v1","source_id":"llvm-langref","source_name":"LLVM Language Reference Manual","repo":"https://github.com/llvm/llvm-project","commit_sha":commit,"retrieved_at":"2026-06-19","license_id":"Apache-2.0 WITH LLVM-exception","files":files}
(out/"source-manifest.json").write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
print(f"downloaded llvm-langref {commit} -> {out}")
for f in files: print(f"  {f['sha256']}  {f['path']}  {f['bytes']} bytes")
PY
