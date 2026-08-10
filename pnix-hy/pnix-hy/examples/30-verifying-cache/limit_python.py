"""한계: 검사(check)를 매번 전부 다시 돌린다 — 무엇이 바뀌었는지 모른 채.

plain하게는 자가검사 스위트를 실행할 때마다 전부 다시 계산한다. 코드가 하나도 안 바뀌었어도
줄여줄 방법이 없고, '이 결과가 지금 코드 상태에 대한 것'이라는 증거도 남지 않는다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
runs = {"n": 0}
def expensive_check():
    runs["n"] += 1
    return {"ready": True}

# 같은 상태에서 두 번 → 두 번 다 계산 (재사용 없음)
expensive_check(); expensive_check()
print("plain: 같은 상태에서도 검사 2회 실행 →", runs["n"], "회 계산")
assert runs["n"] == 2
print("한계: 상태-해시 기반 재사용도, '이 상태에 대한 결과'라는 증거도 없음.")
