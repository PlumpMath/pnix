"""pnix-hy의 방식 — 같은 내성을 host-direct와 stage7 커널 '양쪽'에서 수행하고 일치를 확인.

full_introspection은 Python 소스의 code object/bytecode/AST/symtable/marshal을 물화한다(host-direct).
introspection_parity는 '같은 내성'을 Hy로 작성된 stage7 커널 안에서도 수행해 host 결과와 일치하는지
확인한다 — 자기구현이 드리프트하면 여기서 잡힌다.

* Hy 1.3.0 proof Python 필요 (`nix develop` / PNIX_HY_PYTHON). 입력은 'Python 소스'다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: E402


# 1) host-direct 내성: code object의 여러 단면을 물화.
full = hm.full_introspection("20 + 22")
print("host-direct 내성 단면:", sorted(full.keys()))
assert {"ast", "bytecode", "code_object", "symtable", "marshal"} <= set(full)

# 2) parity: 같은 내성을 stage7(자기구현) 커널에서도 수행 -> 일치?
par = hm.introspection_parity("20 + 22")
print("substrate: host =", par["host"]["mode"], "| mirror(stage7) 존재:", "mirror" in par)
print("host vs stage7 내성 일치(ready):", par["ready"])
assert par["ready"] is True     # 호스트 관점과 자기구현 커널 관점이 일치

print("\n결론: 자기구현(stage7) 내성이 호스트 내성과 일치함을 증명 -> 자기구현 드리프트를 감지한다.")
