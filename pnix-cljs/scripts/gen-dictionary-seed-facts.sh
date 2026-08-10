#!/usr/bin/env bash
# 프로젝트-소유 사전 seed(corpus/dictionary/*.senses.json) → .px facts 모듈 생성.
#
# ★왜 host 생성: .px 거울은 이 eval 모드에서 builtins.readFile 가 없다(fromJSON 만). 사전 facts 를 .px 가
#   소비하려면 *인라인 .px 데이터*여야 한다. host(IO 허용=Rust/스크립트 역할)가 seed JSON 의 문법 facts 만
#   뽑아 .px 리스트로 박아준다. ★단일 source = seed JSON (이 스크립트로 재생성). 정의문(definition_surface)은
#   버린다(anti-transcript=prose durable 금지, facts-only clean-room: lemma·pos·valency·derives_hada=문법 fact 만).
#   pos 분류 로직은 dictionary-facts.px(.px)에 있고, 이 스크립트는 데이터 전사(IO)만 — 의미 0(인터프리터-우선).
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUT="$ROOT/stdlib/lib/nl/dictionary-seed-facts.px"
python3 - "$ROOT" "$OUT" <<'PY'
import json, sys, glob, os
root, out = sys.argv[1], sys.argv[2]
rows = {}
optional_keys = ("aeo", "aeo_rev", "valency", "derives_hada")
for path in sorted(glob.glob(os.path.join(root, "corpus/dictionary/*.senses.json"))):
    data = json.load(open(path, encoding="utf-8"))
    for r in data.get("rows", []):
        lemma = r.get("lemma", ""); pos = r.get("pos", "")
        if not lemma or not pos:
            continue
        key = (lemma, pos)
        row = rows.setdefault(key, {"lemma": lemma, "pos": pos})
        for k in optional_keys:
            if k not in r:
                continue
            v = r[k]
            if isinstance(v, bool):
                row[k] = row.get(k, False) or v
            elif isinstance(v, str) and v:
                row.setdefault(k, v)
def esc(s):  # .px 문자열 escape (한글은 그대로; 따옴표/역슬래시만)
    return s.replace("\\", "\\\\").replace('"', '\\"')
def px_value(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    return '"%s"' % esc(str(v))
lines = []
lines.append("# stdlib/lib/nl/dictionary-seed-facts.px — GENERATED, do not edit by hand.")
lines.append("# 생성: bash scripts/gen-dictionary-seed-facts.sh  (단일 source = corpus/dictionary/*.senses.json)")
lines.append("# 프로젝트-소유 seed 의 문법 facts(정의문 제외=clean-room). dictionary-facts.px 가 소비. host=전사(IO), 의미 0.")
lines.append("{")
lines.append('  schema = "dictionary.seed-facts.v1";')
lines.append("  facts = [")
for row in rows.values():
    attrs = ['lemma = "%s"' % esc(row["lemma"]), 'pos = "%s"' % esc(row["pos"])]
    for k in optional_keys:
        if k in row:
            attrs.append("%s = %s" % (k, px_value(row[k])))
    lines.append("    { %s; }" % "; ".join(attrs))
lines.append("  ];")
lines.append("}")
open(out, "w", encoding="utf-8").write("\n".join(lines) + "\n")
print("generated %s (%d facts)" % (out, len(rows)))
PY
