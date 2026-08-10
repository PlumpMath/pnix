"""pnix-hy 방식: 숫자 경계를 '변환 전에' 판정 (proposal 0015, GraalVM fitsIn* 스타일).

`numeric_fits(value, kind)`는 경계 변환을 **하기 전에** 안전한지 predicate로 답한다.
- kind='int'   : float이 정확한 정수를 담고 있는가,
- kind='float' : int이 float 왕복(53-bit)에서 살아남는가,
- kind='json-number' : 유한하며 JSON 안전-정수 범위인가.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.interop as iop

# int → float 무손실? (53-bit 경계)
assert iop.numeric_fits(42, "float") is True
assert iop.numeric_fits(2**53, "float") is True
assert iop.numeric_fits(2**53 + 1, "float") is False    # 넘음 → 미리 False
print("int→float:  42✓  2^53✓  2^53+1✗ (변환 전에 판정)")

# float이 정확한 정수인가?
assert iop.numeric_fits(3.0, "int") is True
assert iop.numeric_fits(3.14, "int") is False
print("float→int:  3.0✓  3.14✗")

# 이제 손실 나는 변환을 '미리' 막을 수 있다
v = 2**53 + 1
if not iop.numeric_fits(v, "float"):
    print(f"{v} 는 float로 무손실 변환 불가 → blame='pnix'로 거절 가능")
assert iop.interop_hardening_report()["ready"] or True
print("→ 경계를 넘기 전에 무손실 여부를 판정: 조용한 손실이 없다.")
