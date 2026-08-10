#!/usr/bin/env bash
set -euo pipefail

# WebAssembly/spec interpreter OCaml 원천 → pnix attrset source 생성.
# 기관 원천의 구조 필드만 추출한다. raw/prose/comment 본문, 실행/graph 연결 없음.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IN="$ROOT/ingest/code/wasm"
OUT="$ROOT/stdlib/lib/corpus/wasm-spec.generated.px"
MNEMONIC_LIMIT="${PNIX_WASM_MNEMONIC_LIMIT:-256}"
OPCODE_LIMIT="${PNIX_WASM_OPCODE_LIMIT:-256}"

python3 - "$IN" "$OUT" "$MNEMONIC_LIMIT" "$OPCODE_LIMIT" <<'PY'
import hashlib, json, pathlib, re, sys
src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
mnemonic_limit = int(sys.argv[3])
opcode_limit = int(sys.argv[4])
files = ["syntax/types.ml", "syntax/ast.ml", "syntax/mnemonics.ml", "binary/encode.ml", "binary/decode.ml"]
manifest_path = src / "source-manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8")) if manifest_path.exists() else {}

def pnix(v):
    if v is None: return "null"
    if isinstance(v, bool): return "true" if v else "false"
    if isinstance(v, int): return str(v)
    if isinstance(v, str): return json.dumps(v, ensure_ascii=False).replace("${", "\\${")
    if isinstance(v, list): return "[ " + " ".join(pnix(x) for x in v) + " ]"
    if isinstance(v, dict): return "{ " + " ".join(f"{k} = {pnix(v[k])};" for k in sorted(v.keys())) + " }"
    raise TypeError(type(v))

def read(rel):
    return (src / rel).read_text(encoding="utf-8", errors="replace").splitlines()

source_files = []
for rel in ["LICENSE"] + files:
    p = src / rel
    b = p.read_bytes()
    source_files.append({"path": rel, "sha256": hashlib.sha256(b).hexdigest(), "bytes": len(b), "lines": len(b.decode('utf-8', 'replace').splitlines())})

type_defs = []
variants = []
for rel in ["syntax/types.ml", "syntax/ast.ml"]:
    current = ""
    for i, line in enumerate(read(rel), 1):
        s = line.strip()
        m = re.match(r"^(?:type|and)\s+([^=]+?)\s*=", s)
        if m:
            current = m.group(1).strip()
            type_defs.append({"file": rel, "line": i, "name": current})
        vm = re.match(r"^\|\s*([A-Z][A-Za-z0-9_']*)\b", s)
        if vm:
            variants.append({"file": rel, "line": i, "type": current, "name": vm.group(1)})

all_mnemonics = []
for i, line in enumerate(read("syntax/mnemonics.ml"), 1):
    s = line.strip()
    m = re.match(r"^let\s+([a-zA-Z_][A-Za-z0-9_']*)\b(.*?)=", s)
    if m and m.group(1) != "at_const":
        all_mnemonics.append({"file": "syntax/mnemonics.ml", "line": i, "name": m.group(1), "args": " ".join(m.group(2).split())})

all_opcodes = []
hex_re = re.compile(r"(?<![A-Za-z0-9_])-?0x[0-9a-fA-F][0-9a-fA-F_]*(?:l|L)?")
for rel in ["binary/encode.ml", "binary/decode.ml"]:
    for i, line in enumerate(read(rel), 1):
        toks = hex_re.findall(line)
        if not toks: continue
        s = line.strip()
        kind = ""
        if "op 0x" in s or "vecop" in s: kind = "encode_opcode"
        elif re.match(r"^\|\s*0x", s): kind = "decode_opcode"
        elif "section " in s: kind = "section_id"
        elif "s7 (-0x" in s or re.match(r"^\|\s*-0x", s): kind = "type_code"
        if kind:
            all_opcodes.append({"file": rel, "line": i, "kind": kind, "hex": toks})

module_sections = []
section_name = None
for i, line in enumerate(read("binary/encode.ml"), 1):
    cm = re.match(r"\s*\(\*\s*(.+?) section\s*\*\)", line)
    if cm:
        section_name = cm.group(1).strip().lower().replace(" ", "_")
    sm = re.search(r"section\s+(\d+)\s+", line)
    if sm and section_name:
        module_sections.append({"file": "binary/encode.ml", "line": i, "id": int(sm.group(1)), "name": section_name})
        section_name = None
if not any(x["id"] == 0 for x in module_sections):
    module_sections.insert(0, {"file": "binary/encode.ml", "line": 1122, "id": 0, "name": "custom"})

payload = {
    "schema": "code.wasm.spec.v1",
    "source": {
        "project": "WebAssembly specification",
        "repo": manifest.get("repo", "https://github.com/WebAssembly/spec"),
        "commit_sha": manifest.get("commit_sha", ""),
        "retrieved_at": manifest.get("retrieved_at", "2026-06-19"),
        "license_id": "Apache-2.0",
        "scope": "interpreter structural rows only; bounded overlay; no raw prose/examples; no graph wiring",
    },
    "attribution": "WebAssembly specification/interpreter, Apache-2.0, https://github.com/WebAssembly/spec",
    "source_files": source_files,
    "type_definitions": type_defs,
    "variant_constructors": variants,
    "mnemonic_bindings": all_mnemonics[:mnemonic_limit],
    "opcode_literals": all_opcodes[:opcode_limit],
    "module_sections": sorted(module_sections, key=lambda x: x["id"]),
    "counts": {
        "source_files": len(source_files),
        "type_definitions": len(type_defs),
        "variant_constructors": len(variants),
        "mnemonic_bindings": min(len(all_mnemonics), mnemonic_limit),
        "opcode_literals": min(len(all_opcodes), opcode_limit),
        "module_sections": len(module_sections),
    },
    "total_counts": {
        "mnemonic_bindings": len(all_mnemonics),
        "opcode_literals": len(all_opcodes),
    },
    "limits": {"mnemonic_bindings": mnemonic_limit, "opcode_literals": opcode_limit},
}
out.write_text("# GENERATED by scripts/gen-code-wasm-spec.sh. Do not edit.\n" + pnix(payload) + "\n", encoding="utf-8")
print(f"generated {out}: " + ", ".join(f"{k}={v}" for k, v in payload["counts"].items()) + f"; totals mnemonic={len(all_mnemonics)} opcode={len(all_opcodes)}")
PY
