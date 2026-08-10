# 03 · specialization (Futamura) — 부분평가와 잔여 코드 생성

## 쉽게 말하면 (비유)
요리 **밑준비(mise en place)**. 고정 재료(`a=1`)를 미리 손질해 더 짧은 레시피 `(+ 10 x)`를
만들어 둔다 — 손님 올 때마다 처음부터 다시 안 한다.
```py
ph.specialize_pnix("let a = 1; in a * 10 + x", ("x",))["residual_hy"]   # (+ 10 x)
```
직관: 고정 입력을 미리 접어 **더 단순한 프로그램을 코드로** 생성(해석 오버헤드 제거).

## 무엇을
정적 부분을 접어(fold) **더 단순한 잔여 프로그램(residual)**을 코드로 생성한다 (Futamura 1차 사영).

## plain의 한계 (`limit_python.py`)
`functools.partial`은 인자를 기억하는 호출 래퍼일 뿐, 정적 부분을 접어 더 단순한 **소스**를
만들지 않는다. Python에는 소스-수준 부분평가기가 내장돼 있지 않다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `specialize_pnix(src, dynamic_vars)` — `dynamic_vars`만 남기고 나머지를 상수로 접어
  `residual_hy`(예: `(+ 10 x)`)를 생성. 닫힌 프로그램은 값까지 접힘(`fully_static`).
- 결정적 → `(정본 소스, dynamic_vars)`로 메모이즈.

## 어디에 쓰나
- 고정 설정/파라미터로 반복 실행되는 DSL·규칙식: **한 번 특화 → 해석 오버헤드 제거**
- "프로그램 × 고정 입력 → 특화 프로그램" 파이프라인 (컴파일-온스)
- 정적 최적화/상수 접힘을 값-보존과 함께 증명하고 싶을 때 (`specialization_roundtrip` 참고)

## 실행
```sh
python pnix-hy/examples/03-specialization-futamura/limit_python.py
python pnix-hy/examples/03-specialization-futamura/pnix_hy_way.py
```
