"""pnix-hy의 방식 — reify_pnix: 한 폼을 모든 단면으로 통일 물화(+ singleton mirror).

reify_pnix는 하나의 pnix 소스를 source/form/ast/ir/effect/value/witness + mirror facet 으로
한꺼번에, 같은 정본/해시 규약으로 물화한다. (별도 런타임을 새로 만드는 게 아니라 기존
singleton mirror / runtime / IR / interop의 증거를 모은다.) 순수 — Hy 불필요.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


r = ph.reify_pnix("let a = 1; in a + 2")
reified = r["reified"]
print("ok:", r["ok"], "| root tag:", reified["ast"]["root_tag"])
print("단면들:")
print("  source sha256 :", reified["source"]["sha256"][:16], "…")
print("  ast    sha256 :", reified["ast"]["sha256"][:16], "…")
print("  ir     sha256 :", reified["ir"]["sha256"][:16], "…  (IR==AST 해시:", reified["ir"]["sha256"] == reified["ast"]["sha256"], ")")
print("  effect        :", "pure=", reified["effect"]["pure"])
print("  value         :", reified["value"]["data"])
print("  mirror facets :", reified["mirror"]["facet_count"])
print("  witness run_id:", reified["witness"]["run_id"])

assert r["ok"] is True
assert reified["ast"]["root_tag"] == "let"
assert reified["value"]["data"] == 3
assert reified["effect"]["pure"] is True
assert reified["mirror"]["facet_count"] >= 6   # source/token/ast/ir/effect/value/… 한 번에

print("\n결론: 한 폼을 모든 단면으로 '한 규약'으로 물화 -> 감사/디버그/설명이 일관된다.")
