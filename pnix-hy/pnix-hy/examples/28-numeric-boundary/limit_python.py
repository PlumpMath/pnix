"""한계: Python은 숫자 경계 변환의 정밀도 손실을 조용히 넘긴다.

Python에서 큰 int를 float로 바꾸면 정밀도가 조용히 깨진다 — 오류도, 경고도 없다.
경계를 넘기 '전에' 안전한지 물어볼 predicate가 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
big = 2**53 + 1
as_float = float(big)                 # 조용히 정밀도 손실
print(f"int {big} -> float {as_float!r} -> int {int(as_float)}")
print("손실 발생?:", int(as_float) != big)   # True — 그런데 아무 경고 없음
assert int(as_float) != big
print("한계: 변환 전에 '무손실인가'를 물어볼 수단이 없음 — 조용히 깨진다.")
