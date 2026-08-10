"""한계: 이름 기반 캐시는 '의미가 같아도' 이름만 바뀌면 무효화된다.

plain Python(또는 소스-문자열 캐시)은 정의의 정체성을 **이름/텍스트**로 본다. 그래서
- 무거운 정의 하나만 바뀌어도 전체를 다시 계산하거나,
- 의미가 전혀 안 바뀐 alpha-rename(변수 이름만 교체)에도 캐시가 통째로 miss 난다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

# 소스-문자열을 키로 쓰는 순진한 캐시
_cache = {}
def eval_cached(expr):
    if expr in _cache:
        return _cache[expr], "hit"
    v = eval(expr, {"__builtins__": {}})   # 데모용
    _cache[expr] = v
    return v, "miss"

a = "1000*1000 + 1 + 8"
b = "1000*1000 + 1 + 9"                    # other만 8->9 (무거운 1000*1000은 그대로)
print("전체 재계산:", eval_cached(a), eval_cached(b))   # b는 전체 miss (부분 재사용 없음)

# alpha-rename: 의미가 같은데 텍스트가 달라 캐시 miss
r1 = "(lambda big: big + 1)(1000*1000)"
r2 = "(lambda huge: huge + 1)(1000*1000)"  # big -> huge, 의미 동일
_, s1 = eval_cached(r1)
_, s2 = eval_cached(r2)
print("alpha-rename:", s1, s2)             # miss, miss — 이름만 바뀌어도 재계산
assert s2 == "miss"
print("한계: 이름/텍스트가 정체성 → 의미 불변 변경도 캐시 무효화 + 부분 재사용 없음.")
