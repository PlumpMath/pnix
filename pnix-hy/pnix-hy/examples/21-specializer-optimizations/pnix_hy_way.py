"""pnix-hy 방식: 부분평가기의 binding-time-improvement(BTI) 계열 (딥리서치 #2/#3 도출).

`tower.poly_specialize`(hand-written 폴리variant specializer, 0029 cogen approach의 엔진)는 4가지
BTI로 잔여를 작고 빠르게 유지한다 — 전부 순수 pnix라 의미 불변, 크기/공유만 개선:
  Q1-1 sharing-safe unfolding — dynamic 바인딩 다회 사용 시 공유 let(인라인 복제 금지)
  Q1-2 eta "The Trick"        — dynamic if를 구조(attrset/list)로 분배해 정적 필드 폴딩
  I4   let-insertion          — 분배로 생길 cond 중복을 최상위 공유 let으로 hoist
  0030 commuting conversion   — 주변 연산을 dynamic if 브랜치로 밀어 폴딩(Bondorf CPS 효과)
  I1   bounded static variation — 위 분배가 크게 복제될 상황은 예산으로 억제
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402
import pnix_hy.tower as tw  # noqa: E402
import pnix_hy.pnix_runtime as rt  # noqa: E402


def spec(src, dyn):
    return tw.poly_specialize(src, dyn)["residual"]


# Q1-1 sharing: dynamic subexpr는 사용 횟수와 무관하게 1회만 (공유 let)
r = spec("let y = x * x + 7; in y + y", ("x",))
print("Q1-1 sharing        :", r)
assert r.count("x * x") == 1 and rt.eval_source("let x = 5; in " + r) == (25 + 7) * 2

# Q1-2 eta "The Trick": dynamic if를 구조로 분배 → 정적 필드 폴딩(attrset 소거)
r = spec("(if b then { v = 1; } else { v = 2; }).v", ("b",))
print("Q1-2 eta trick      :", r)
assert "{" not in r and rt.eval_source("let b = true; in " + r) == 1

# I4 let-insertion: 분배 시 non-trivial cond를 1회 hoist(중복 방지)
r = spec("let r = if (x * x + x) > 5 then { a = 1; c = 2; } else { a = 3; c = 4; }; in r.a + r.c", ("x",))
print("I4 let-insertion    :", r)
assert r.count("x * x") == 1 and rt.eval_source("let x = 5; in " + r) == 3

# 0030 commuting conversion: 주변 연산을 if 브랜치로 밀어 폴딩(Bondorf CPS 효과)
r = spec("(if d then 10 else 20) + 5", ("d",))
print("0030 commuting      :", r)
assert "+" not in r and rt.eval_source("let d = true; in " + r) == 15

# I1 bounded: dynamic 다른-피연산자는 밀지 않음(중복 폭증 방지)
r = spec("(if d then 10 else 20) + e", ("d", "e"))
print("I1 bounded (no push):", r)
assert "+" in r

# 정량 게이트: 공유 부분식은 사용 k회여도 1회 (naive는 k회 복제)
sizes = ph.pe_size_report()
assert sizes["ready"] and sizes["sharing_subexpr_count_constant"]
print("pe_size: 공유 부분식 사용 k=2/4/8/16 전부 1회 (naive 복제 k회) →", sizes["sizes_by_uses"][2]["residual_len"],
      "..", sizes["sizes_by_uses"][16]["residual_len"], "bytes")
print("→ 같은 의미, 더 작은 잔여: research-backed BTI 계열.")
