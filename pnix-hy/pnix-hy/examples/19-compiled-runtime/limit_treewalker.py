"""한계: 순수 tree-walker 평가 (plain 인터프리터의 해석 오버헤드).

plain하게 프로그램을 평가하는 인터프리터(tree-walker)는 매 노드마다 "이게 무슨 tag인가"를 다시
판별(dispatch)하며 돈다. 순수·재귀가 많은(hot) 코드에서는 이 해석 오버헤드가 그대로 쌓인다.
게다가 pnix 기본 평가 경로는 호스트(파이썬) 스택에 얹혀, 깊은 재귀는 재귀한계에 부딪힌다.

여기서는 pnix의 정본 평가기(pnix_runtime.eval_source = tree-walker)로 fib를 돌려 그 특성을 본다.
"""
import os
import sys
import time

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.pnix_runtime as rt  # noqa: E402

FIB = "let fib = n: if n < 2 then n else (fib (n - 1)) + (fib (n - 2)); in fib 22"

t0 = time.perf_counter()
value = rt.stable_data(rt.eval_source(FIB))       # 정본(canonical) 결과 — 이게 진실의 기준
dt = time.perf_counter() - t0
print(f"tree-walker: fib 22 = {value}  ({dt:.3f}s)")

# 한계 1: 매 노드 재-dispatch로 hot 코드가 느리다 (아래 pnix_hy_way와 비교).
# 한계 2: 깊은 재귀는 호스트 스택 재귀한계에 걸린다 — 기본 재귀한계에선 fib 30, countdown 5000 등이 터진다.
assert value == 17711  # fib 22
print("이 결과가 정본(canonical) — 컴파일 런타임은 반드시 이것과 같아야 한다.")
