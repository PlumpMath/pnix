"""plain의 한계 — Python엔 매크로가 없고, pnix는 동형(homoiconic)이 아니다.

  - Python: 함수는 인자를 '먼저 평가'한다. 컴파일 전에 코드를 '데이터로' 받아 위생적으로
    변환하는 1급 매크로 시스템이 없다(문자열/ast 해킹은 1급이 아니다).
  - pnix: 순수·비동형 언어라 quote/quasiquote/macro 자체가 없다(설계상).
따라서 '어느 한쪽 언어만으로는' 매크로를 pnix 코드 위에 적용할 수 없다.
"""

# Python 함수는 인자를 먼저 평가한다 — 'when(cond, expr)'을 함수로 만들면 expr이 항상 평가된다.
def when(cond, expr):
    return expr if cond else None

log = []
when(False, log.append("이 부작용은 일어나면 안 되는데..."))  # 함수라서 expr이 이미 평가됨!
print("함수 인자 선평가로 부작용 발생:", log)  # -> ['...'] : 매크로였다면 막혔을 것

print("Python: 컴파일 전 코드-변환용 1급 매크로 = 없음")
print("pnix:  quote/quasiquote/macro = 없음 (비동형, 설계상)")
print("\n결론: 한쪽 언어만으로는 'pnix 코드 위에 매크로 적용'이 불가능하다.")
