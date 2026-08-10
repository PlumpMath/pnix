# 09 · mirror & reify — 한 폼을 모든 단면으로 통일 물화

## 쉽게 말하면 (비유)
**X-ray/MRI**. 한 프로그램을 소스/토큰/AST/IR/값/부작용/witness 등 **여러 단면으로 한 번에 촬영**한다.
```py
ph.reify_pnix("let a = 1; in a + 2")["reified"].keys()   # source, ast, ir, effect, value, witness, mirror ...
```
직관: 내부 구조를 **한 규약으로 통째로 관찰** → 감사/디버그/설명이 일관된다.

## 무엇을
하나의 프로그램을 **source / form / ast / ir / effect / value / witness + mirror facet**으로
**한 규약(정본·해시)으로 한꺼번에** 물화(reify)한다. (순수 — Hy 불필요.)

## plain의 한계 (`limit_python.py`)
Python에선 `ast`/`dis`/`symtable`/`inspect`/`hashlib`를 **따로따로** 호출해 직접 꿰매야 하고,
각 단계가 같은 정본/해시 규약을 공유하지 않는다 → 일관된 물화 표면이 없다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `reify_pnix(src)` — 한 폼의 모든 단면 + singleton mirror facet을 같은 규약으로 물화
  (IR 해시 == AST 해시, effect 순수성, value, witness run_id 등).
- 새 런타임을 만드는 게 아니라 기존 mirror/runtime/IR/interop의 증거를 모은다.

## 어디에 쓰나
- **감사/디버그/설명(explain)**: 한 번의 호출로 프로그램의 모든 단면을 일관되게 확보
- 파이프라인 각 단계의 해시를 한 witness로 묶어 재현/추적
- (심화) `pnix_meta_circular_projection` = 같은 폼을 4개 substrate에서 평가·수렴 (섹션 11 참고)

## 실행
```sh
python pnix-hy/examples/09-mirror-and-reify/limit_python.py
python pnix-hy/examples/09-mirror-and-reify/pnix_hy_way.py
```
