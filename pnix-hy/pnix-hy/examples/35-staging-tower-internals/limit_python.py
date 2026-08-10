"""한계: 평범한 평가는 '한 번에 통째로' — 멈추고·스냅샷하고·재개할 수 없다.

Python이 식을 평가할 때 그 계산은 호스트 콜스택에 갇혀 있다. 도중에 멈춰서 상태를 데이터로 꺼내(reify)
해시·증거를 남기고, 나중에 그 지점부터 이어서 재개하는 건 불가능하다. 또 '같은 인터프리터'가 값을
내거나(interpret) 코드를 내는(compile) 두 역할을 겸하지도 못한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
def evaluate(expr):
    return eval(expr, {"__builtins__": {}})   # 통째로, 중간 상태 없음
print("plain eval:", evaluate("(2 + 3) * 4"))
print("한계: 중간에 멈춤·reify·재개 불가; interpret/compile 겸용도 불가; 정적/동적 분류 없음.")
