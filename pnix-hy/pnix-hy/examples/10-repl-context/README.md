# 10 · REPL context — warm, 컨텍스트 유지 대화형

## 쉽게 말하면 (비유)
**불 켜둔 작업실 vs 매번 전원 껐다 켜기**. warm REPL은 작업대 위 물건(바인딩)이 그대로 남아
빠르다. 반복 CLI(`python -c`)는 매번 재부팅이라 이전 상태가 사라진다.
```py
# REPL 한 세션:  a = 20  ->  b = a + 22  ->  b   => 42  (a를 기억한다)
```
직관: **컨텍스트 유지 + warm이라 빠름** → 대화형 개발.

## 무엇을
한 프로세스에서 **바인딩이 누적되는** pnix REPL. `a = 20` → `b = a + 22` → `b` = 42.

## plain의 한계 (`limit_python.py`)
반복 `python -c` 호출은 **stateless** — 매번 새 프로세스라 이전 바인딩이 사라지고, 매 호출이
인터프리터를 재기동해 느리다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- pnix REPL은 **warm 프로세스**로 pnix env를 누적(context 유지) → 반복 CLI보다 빠름.
- `_` = 직전 값, `name = expr`/`:let`, `:env`/`:reset`/`:help`/`:quit`, 오류 줄은 진단 후 세션 유지.
- 헤드리스: `run_pnix_repl(inp, out)`; 대화형: `nix run .#pnix-hy-pnix` /
  `pnix-hy-project --repl pnix` (python/hy 모드도 있음 — 저장소 README 참고).

## 어디에 쓰나
- pnix/Hy/Python을 **대화형으로 탐색**하며 상태를 쌓아가는 작업(디버깅/실험)
- "REPL은 느리다"의 반대: warm 프로세스라 반복 호출보다 빠르다

## 실행
```sh
python pnix-hy/examples/10-repl-context/limit_python.py
python pnix-hy/examples/10-repl-context/pnix_hy_way.py     # 헤드리스 검증
nix run .#pnix-hy-pnix                                     # 대화형
```
