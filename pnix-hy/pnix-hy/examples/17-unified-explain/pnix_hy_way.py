"""pnix-hy의 방식 — explain_pnix: 값·순수성·안전평가·진단·단면을 '한 번에'.

explain_pnix는 한 소스에 대해 purity(순수성) + safe_eval(자원제한 평가/값) + mirror(단면) +
diagnostic(진단)을 하나의 레코드로 묶어 돌려준다. 순수 — Hy 불필요.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


e = ph.explain_pnix("let a = 1; in a + 2")
print("ok:", e["ok"], "| phase:", e["phase"])
print("purity.pure   :", e["purity"]["pure"])
print("safe_eval.value:", e["safe_eval"]["value"], "| ok:", e["safe_eval"]["ok"])
print("mirror facets :", len(e["mirror"].get("facets", [])) if isinstance(e.get("mirror"), dict) else "-")

assert e["ok"] is True
assert e["purity"]["pure"] is True
assert e["safe_eval"]["value"] == 3 and e["safe_eval"]["ok"] is True

# 권한이 필요한 계산도 한 번에 설명된다 (granted로 권한 부여 가능).
imp = ph.explain_pnix('builtins.readFile "/etc/passwd"')
print("\nimpure 설명: purity.pure =", imp["purity"]["pure"], "| impure_uses:", imp["purity"]["impure_uses"])
assert imp["purity"]["pure"] is False

print("\n결론: 값·순수성·안전성·진단·단면을 한 호출로 통합 설명 -> 감사/디버그/UX에 바로 쓴다.")
