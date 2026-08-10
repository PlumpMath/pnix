# 19. 컴파일 런타임 — 같은 답을 더 빠르게 (proposal 0028 P1)

## 무엇을
pnix core-subset를 **Python 클로저로 컴파일**해 실행하는 `compiled_eval`. 정본(`pnix_runtime`,
tree-walker)과 **결과는 같고**, hot·재귀 코드에서 **더 빠르다**.

## 왜
tree-walker는 매 노드마다 "무슨 tag인가"를 다시 판별(dispatch)한다. 컴파일 런타임은 그 판별을
**한 번만**(컴파일 시) 하고, 이후엔 Python 클로저를 바로 호출한다 → 해석 오버헤드 제거.
게다가 깊은 재귀를 큰 스택 워커 스레드에서 돌려 tree-walker의 기본 재귀한계도 넘는다.

## 쉽게 말하면 (비유)
```
tree-walker  = 요리할 때마다 레시피를 처음부터 다시 읽으며 만든다
compiled     = 레시피를 한 번 읽고 "동작 순서"로 접어둔 뒤, 그대로 빠르게 반복한다
결과 요리는 똑같아야 한다 — compiled_runtime_report가 코퍼스 전수로 그 동일성을 게이트한다.
```

## 어디에 쓰나
- **hot·재귀가 많은 순수 core-subset 계산**(fib류, 누적 재귀, map 반복)을 빠르게.
- 정본(`pnix_runtime`)은 **진실의 기준**으로 두고, 컴파일 런타임은 **대조 lane**으로 검증하며 가속.

## 실측 (이 저장소, `pnix-hy-project --compiled-bench`)
| 케이스 | tree-walker | compiled | 속도 |
|---|---|---|---|
| fib 22 | ~1.3s | ~0.16s | **~8x** |
| countdown 3000 | ~0.25s | ~0.017s | **~15x** |
| map/length ×2000 | ~1.6s | ~0.83s | ~2x |
| small arith ×4000 | ~1.0s | ~0.30s | ~3.5x |

## 코드 발췌
```python
import pnix_hy as ph, pnix_hy.pnix_runtime as rt
FIB = "let fib = n: if n < 2 then n else (fib (n-1)) + (fib (n-2)); in fib 22"
assert ph.compiled_eval(FIB) == rt.stable_data(rt.eval_source(FIB))   # 같은 답
# 깊은 재귀도 OK (tree-walker 기본 재귀한계 초과):
assert ph.compiled_eval("let f = n: if n==0 then 0 else 1 + (f (n-1)); in f 5000") == 5000
```

## 한 줄
> "그냥 실행"(tree-walker, 매번 재판별)과 "**한 번 컴파일 후 빠르게 실행**"(정본과 동일성 게이트된
> 대조 lane)의 차이 — 순수 core-subset 코드에 한해 후자가 크게 빠르다.

## 경계 (정직)
- 지원 범위 = **core subset**(int/bool/string/null·var·연산·if·let·lambda·apply·select·attrset·
  list·일부 builtins). 나머지(rec attrset·with·string-interp·경로·전체 builtins·예외 등)는 정본으로.
- 정본은 언제나 `pnix_runtime` (SACRED). compiled은 그것을 대체하지 않는다.
