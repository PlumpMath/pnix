"""한계: naive cogen (self-application으로 만든 3차 Futamura 사영).

cogen(compiler generator)을 "특화기를 자기 자신에 특화"(self-application, mix^3)로 만들면 개념적으로는
맞지만, 만들어진 아티팩트가 **병리적으로 비대**하다: 자기적용된 특화기가 인터프리터 + universal value
자료형 + 환경/태그 조작을 생성확장마다 끌고 들어간다. 그래서 이 cogen으로 **인터프리터→풀 컴파일러**를
재도출하려 하면, num-only 1-branch 초소형 인터프리터조차 >150초로 사실상 실행 불가다(런타임/규모 무관,
본 저장소 4실험 + PE 문헌이 일치: Birkedal&Welinder'94, Thiemann'96, JGS §4.8/§7.3).

여기서는 self-application cogen(`tower.run_cogen`)을 **작은 입력**에만 돌려 "동작은 하지만 확장이 안 됨"을
보인다. (풀 컴파일러 경로는 의도적으로 실행하지 않는다 — >150초이므로.)
"""
import os
import sys
import time

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.tower as tw  # noqa: E402

# self-applied cogen을 작은 입력에 실행 — 특화기로서 동작은 한다
t0 = time.perf_counter()
r = tw.run_cogen("a * b", {"a": 6})
dt = time.perf_counter() - t0
print(f"naive run_cogen(a*b, a=6) = {r['residual']}  ({dt:.2f}s)")
assert r["residual"] == "(6 * b)"

# 한계: 이 self-application 아티팩트로 인터프리터→풀 컴파일러를 재도출하면 >150초(비현실적).
# 그래서 여기서는 실행하지 않는다. 근거: docs/audits/2026-07-02-cogen-stagepoly-research.md
print("한계: self-application cogen은 인터프리터→풀 컴파일러 재도출이 >150초로 확장 불가.")
print("→ 올바른 길은 pnix_hy_way.py 의 'cogen approach'(hand-written 생성기).")
