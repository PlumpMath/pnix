"""pnix-hy 방식: 컴파일 런타임(`compiled_eval`)으로 같은 결과를 더 빠르게 (proposal 0028 P1).

`pnix_hy.compiled_eval`은 pnix core-subset AST를 **한 번** Python 클로저 트리로 컴파일한 뒤 실행한다
(매 노드 재-dispatch 없음). linked-env 프레임 + memoized thunk + native 연산으로, hot·재귀 코드에서
tree-walker보다 크게 빠르다. 깊은 재귀는 큰 스택 워커 스레드에서 돌려 재귀한계도 넘는다.

핵심: 컴파일 런타임은 **대조 lane**이다 — 정본은 여전히 `pnix_runtime`이고, `compiled_runtime_report`가
코퍼스 전수 동등을 게이트한다. "빠르지만 다른 답"은 허용되지 않는다.
"""
import os
import sys
import time

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402
import pnix_hy.pnix_runtime as rt  # noqa: E402

FIB = "let fib = n: if n < 2 then n else (fib (n - 1)) + (fib (n - 2)); in fib 22"

canonical = rt.stable_data(rt.eval_source(FIB))     # 정본 기준
t0 = time.perf_counter()
value = ph.compiled_eval(FIB)                        # 컴파일 런타임
dt = time.perf_counter() - t0
print(f"compiled_eval: fib 22 = {value}  ({dt:.3f}s)")

# (1) 같은 답: 컴파일 런타임은 정본과 반드시 일치
assert value == canonical == 17711

# (2) 깊은 재귀도 OK: tree-walker가 기본 재귀한계에서 터지는 깊이도 컴파일 런타임은 큰 스택에서 처리
deep = ph.compiled_eval("let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f 5000")
assert deep == 5000
print(f"deep recursion (f 5000) = {deep}  (tree-walker 기본 재귀한계 초과 깊이)")

# (3) 코퍼스 전수 동등이 게이트로 보장됨 (--check의 compiled_runtime 리포트)
report = ph.compiled_runtime_report()
assert report["ready"] and report["agree"] == report["corpus"]
print(f"compiled_runtime_report: {report['agree']}/{report['corpus']} 코퍼스 동등 (정본과 일치)")

# 벤치마크로 실제 속도 우위 확인 (fib/재귀에서 큰 배수)
bench = ph.compiled_bench(time.perf_counter)
fib_row = next(r for r in bench["rows"] if r["case"].startswith("fib"))
assert bench["all_agree"] and fib_row["speedup"] and fib_row["speedup"] > 1.0
print(f"speedup(fib) = {fib_row['speedup']}x  (전 케이스 결과 동일: {bench['all_agree']})")
print("→ hot·재귀 순수 core-subset 코드는 compiled_eval로 빠르게 (정본 검증된 대조 lane).")
