"""한계: plain eval에는 '컴파일 단계 vs 실행 단계' 구분이 없다.

Python의 `eval`/매크로류는 코드 생성(컴파일 시)과 실행(런타임)을 대수적으로 구분하지 않는다. 그래서
- quote/unquote 같은 스테이징 연산이 서로 상쇄되는지(±1) 추적할 수 없고,
- '소스를 IR로 낮추는(lowering)' 일이 런타임 상태를 건드렸는지 보장할 수 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

# 스테이징을 phase 정수로 추적하지 않으면, 이런 상쇄를 확인할 방법이 없다:
staging = ["quote", "unquote"]          # 개념상 +1 그리고 -1 → 0 이어야
# plain Python엔 이걸 계산할 대수가 없다:
print("plain: staging 연산의 phase 합? — 추적 수단 없음:", staging)
print("한계: compile/run 단계 분리도, lowering이 순수한지도 대수적으로 보장 불가.")
