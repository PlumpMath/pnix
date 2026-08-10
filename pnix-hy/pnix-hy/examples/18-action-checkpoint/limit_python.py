"""plain의 한계 — 한 action step의 안전/효과/의미/증거 verdict를 직접 묶어야 한다.

eval은 값 하나를 돌려줄 뿐이다. "이 행동은 accepted인가, 파일 권한 때문에 held인가,
문법 오류라 rejected인가, witness/rollback hash는 무엇인가" 같은 판정 레코드는 없다.
"""

src = "1 + 2"
value = eval(src)
print("값:", value)
print("accepted/held/rejected verdict?: 없음")
print("effect gate?: 없음")
print("witness_id / rollback_ref?: 없음")

print("\n결론: 값은 얻지만, 행동 승인에 필요한 의미·효과·증거를 한 레코드로 고정하지 못한다.")
