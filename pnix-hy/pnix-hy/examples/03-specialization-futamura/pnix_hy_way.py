"""pnix-hy의 방식 — specialize_pnix = Futamura 1차 사영(부분평가/잔여 코드 생성).

정적 부분을 실제로 접어서(fold) 더 단순한 '잔여 코드'를 만든다. 지정한 dynamic 변수만
남기고 나머지는 상수로 계산해 버린다. 결정적이라 (정본 소스, dynamic vars)로 메모이즈된다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# a=1은 정적, x는 동적 -> 잔여 코드는 '(+ 10 x)' 로 접힌다.
spec = ph.specialize_pnix("let a = 1; in a * 10 + x", ("x",))
print("잔여 코드(residual):", spec["residual_hy"])   # (+ 10 x)  <- 실제 '코드'
print("완전 정적?:", spec["fully_static"])           # False (x가 남아 있으므로)
assert spec["residual_hy"] == "(+ 10 x)" and spec["fully_static"] is False

# dynamic 변수가 없으면(닫힌 프로그램) 값까지 완전히 접힌다.
closed = ph.specialize_pnix("let a = 1; in a * 10 + 5", ())
print("닫힌 프로그램 -> 값으로 접힘:", closed["fully_static"], "| 값:", closed.get("value"))
assert closed["fully_static"] is True and closed.get("value") == 15

# 결정성: 같은 (소스, dynamic vars)는 캐시된다.
again = ph.specialize_pnix("let a = 1; in a * 10 + x", ("x",))
print("메모이즈(cached):", again.get("cached"))

print("\n결론: 고정 입력에 특화된 잔여 프로그램을 '코드로' 생성 -> 해석 오버헤드 제거(1회 컴파일).")
