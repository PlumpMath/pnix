#!/usr/bin/env bash
# Thin valency overlay:
#   current kernel evidence comes from existing Korean NL owner lists, not from raw dictionary prose.
#   Output is gitignored and additive only.
set -euo pipefail

ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUT="${ROOT}/stdlib/lib/nl/dictionary-valency-overlay.generated.px"

python3 - "$OUT" <<'PY'
from pathlib import Path

out = Path(__import__("sys").argv[1])

transitive = ["먹", "읽", "열", "사", "보", "쓰", "마시", "듣"]
intransitive = ["있", "계시", "없", "가", "오"]

def emit_map(name, items):
    rows = "\n".join(f'    {{ name = "{x}"; value = "{name}"; }}' for x in items)
    return rows

txt = """# GENERATED (gitignored), do not commit.
{ schema = "dictionary.valency-overlay.v1";
  overlay = true;
  source = "thin valency from existing Korean NL owner lists";
  records = [
%s
%s
  ];
}
""" % (emit_map("transitive", transitive), emit_map("intransitive", intransitive))
out.write_text(txt, encoding="utf-8")
print(f"generated valency overlay: transitive={len(transitive)} intransitive={len(intransitive)}")
PY
