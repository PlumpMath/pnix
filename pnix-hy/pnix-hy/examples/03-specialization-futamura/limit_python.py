"""plain Python의 한계 — 부분입력에 특화된 '잔여 프로그램'을 만들 수 없다.

`functools.partial`은 인자만 미리 채운 '호출 래퍼'일 뿐, 정적 부분을 접어서(fold) 더 단순한
'잔여 코드(residual program)'를 만들어 주지 않는다. 즉 부분평가(partial evaluation) /
Futamura 사영이 언어에 내장돼 있지 않다.
"""
from functools import partial


def program(a, x):
    # a는 정적(고정), x는 동적(런타임 입력)이라고 하자.
    return a * 10 + x


# partial은 a를 '기억'만 한다 — 여전히 program을 매번 다시 해석/호출한다.
specialized = partial(program, 1)
print("partial 결과:", specialized(5))          # 15
print("잔여 코드는?:", specialized)               # functools.partial 객체 (소스가 아니다)

# 우리가 원한 것: a=1을 접어서 "잔여 프로그램 = (10 + x)"를 '코드로' 얻는 것.
# Python 표준에는 이런 소스-수준 부분평가기가 없다.
print("원하는 잔여 코드 '(+ 10 x)' 를 소스로 얻는 표준 방법: 없음")

print("\n결론: 고정 부분을 접어 더 단순한 프로그램을 '생성'하는 기능이 언어에 없다.")
