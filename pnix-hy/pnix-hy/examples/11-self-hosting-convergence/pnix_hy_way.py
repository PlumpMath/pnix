"""pnix-hy의 방식 — 한 pnix 폼을 4개 substrate에서 평가하고 '수렴'을 증명한다.

같은 pnix 식을:
  1) Python 해석기(host interp),
  2) Python 컴파일러(host compiler),
  3) Hy로 '작성된' pnix 평가기(stage7 runtime),
  4) Hy로 '작성된' pnix 컴파일러(stage7 compiler)
네 경로로 평가해 모두 같은 값으로 수렴하는지 확인한다 — 이것이 meta-circular 자기호스팅의 핵심.

* Hy 1.3.0 proof Python + 저장소 트리 필요 (`nix develop` / PNIX_HY_PYTHON).
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


r = ph.pnix_meta_circular_projection("2 * 3 + 4")
print("소스:", r["source"], "| root:", r["ast_root_tag"])
print("substrate 설명:")
for k, desc in r["substrates"].items():
    print(f"  {k:16s} -> {r['lanes'][k]}   ({desc})")
print("네 경로 수렴(converged):", r["converged"], "| 오류:", r["errors"])

assert r["converged"] is True
assert set(r["lanes"].values()) == {10}          # 네 경로 모두 10
assert set(r["substrates"]) == {
    "host_interp", "host_compiler", "stage7_runtime", "stage7_compiler"
}

print("\n결론: 'Python으로 구현' + 'Hy로 자기 자신을 구현'한 런타임이 같은 값으로 수렴 = 자기호스팅.")
