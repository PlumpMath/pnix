"""plain Python의 한계 — 언어 간 값 변환의 손실/부작용/권한이 '무표시'다.

Python에서 값을 다른 표현으로 바꾸거나 콜러블을 경계 밖으로 넘길 때:
  1) 변환 손실(tuple->list, set->?, bytes->?)이 조용히 일어나고,
  2) 그 값이 '부작용을 일으키는 콜러블'인지 데이터에 표시가 없으며,
  3) 호출 권한(capability)을 제어할 지점이 없다.
"""
import json

# 1) 조용한 손실: tuple은 JSON을 거치면 list가 되지만, 아무도 '손실'이라 말해주지 않는다.
original = (1, 2, 3)
roundtripped = json.loads(json.dumps(original))
print("tuple:", original, "-> json roundtrip ->", roundtripped, type(roundtripped).__name__)
print("손실 표시?:", "없음 (list가 되었지만 lossy라고 알려주지 않는다)")

# set은 아예 직렬화 불가 -> 예외이거나 별도 처리 필요, 역시 '표준 손실 표기'는 없다.
try:
    json.dumps({1, 2})
except TypeError as e:
    print("set 변환:", type(e).__name__, "(손실/불가 표기는 직접 해야 한다)")

# 2) 콜러블을 경계 밖으로 넘길 때: 이게 부작용을 일으키는지 데이터엔 표시가 없다.
def danger():
    return "부작용을 일으킬 수도 있는 호출"
payload = {"fn": danger}         # 그냥 담긴다 — '이건 host-call 권한이 필요하다'는 표시가 없다
print("콜러블 전달:", type(payload["fn"]).__name__, "| 권한 표시?: 없음")

print("\n결론: 변환 손실/부작용/권한이 값 자체에 기록되지 않아, 경계 넘김이 '조용히' 위험하다.")
