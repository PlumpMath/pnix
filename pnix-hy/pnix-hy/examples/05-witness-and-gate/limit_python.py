"""plain Python의 한계 — eval은 '증거(witness)'를 남기지 않고 권한 제어가 없다.

Python으로 무언가를 평가하면:
  1) "이 입력에서 이 출력이 나왔다"는 검증 가능한 내용해시 영수증이 없고,
  2) 키 순서/표현이 달라도 같은 의미인지 안정적으로 확인할 표준이 없으며,
  3) "이 계산은 file-read 권한이 필요하다" 같은 capability 게이트가 없다.
"""
result = eval("2 + 40")
print("결과:", result)
print("영수증(witness)?:", "없음 (무엇으로부터·어떤 환경에서 나왔는지 검증 불가)")

# 같은 의미의 dict라도 키 순서가 다르면 str()은 달라 보인다 -> 안정적 동일성 판단이 번거롭다.
a = {"value": 21, "source": "x"}
b = {"source": "x", "value": 21}
print("키 순서 다른 dict str 비교:", str(a) == str(b), "(내용은 같지만 표현은 다르다)")

# 권한 게이트 없음: eval은 'file-read가 필요한 계산'을 구분/차단하지 않는다.
print("capability 게이트?:", "없음")

print("\n결론: 재현·감사·권한제어에 필요한 witness/gate가 기본 제공되지 않는다.")
