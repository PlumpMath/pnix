# 0030 — context propagation (Bondorf CPS-specializer EFFECT, without a CPS rewrite)

- 상태: **P1 SHIPPED 2026-07-03** (연구백로그 Q1-3). 근거: `docs/audits/2026-07-03-laziness-stagepoly-research.md`.
- Scope: `pnix_hy/tower.py`의 `_ps`(specializer) 확장만. host/pnix specializer lane · 추가적 ·
  `pnix_runtime`/stage7/4-lane **SACRED 무접촉** · 순수 pnix라 의미 불변(크기/폴딩만 개선).

## 배경
Q1-3(백로그)은 "specializer를 CPS로 재작성"(Bondorf LFP'92)해 BTI 효과를 얻는 것. **전면 CPS 재작성은
대규모·위험**(검증된 `_ps` 리스크). 그러나 Bondorf CPS specializer가 주는 **효과 = 문맥 전파**(주변
연산 문맥을 dynamic 제어흐름 안으로 밀어 각 브랜치에서 정적 폴딩 회복)는 **commuting conversion**으로
직접 실현 가능 — CPS 재작성 없이. 이미 shipped된 select-through-if(Q1-2)의 자연스러운 완성.

## 계측(정직)
- **synthetic 제어흐름 프로그램엔 실익**: `(if d then 10 else 20) + 5`가 폴딩 안 되던 것을 `(if d then 15
  else 25)`로.
- **우리 현재 워크로드(인터프리터→컴파일러)엔 미등장**: 인터프리터는 static prog로 분기 → 컴파일러
  잔여에 dynamic-if 피연산자 0개. 따라서 이건 **일반 specializer 강화**(제어흐름 무거운 입력용)이지 현재
  헤드라인 개선은 아님 — 정직히 기록. (pnix-hy 목적=언어 표현력 투영 연구 툴킷이므로 더 강한 specializer
  자체가 능력 향상.)

## P1 — 구현 (commuting conversion for binary ops)
`_ps` binary 핸들러 + `_commute_binary_if`/`_scalar_lit_node`: 한 피연산자가 dynamic `if` AST이고
**다른 피연산자가 static scalar**면 op를 브랜치로 push —
`(if c then a else b) op R` → `if c then (a op R) else (b op R)` (양측 대칭). 각 브랜치가 폴딩되고 static
피연산자는 작은 literal로만 복제. **다른 피연산자가 dynamic이면 push 안 함**(중복 방지 — bounded);
`&&`/`||`(lazy) 제외.

검증(`pe_size_report.commuting_conversion`): `(if d then 10 else 20)+5`→`(if d then 15 else 25)`,
`100 - (if d then 10 else 20)`→`(if d then 90 else 80)`, `(if d then 10 else 20)+e`(dynamic)→push 안 함,
전부 parity. 회귀 0(tower_ladder milestone 6, M4/M5/cogen/pe_size).

## 수용 기준 (충족)
- static-scalar 문맥에서 binary가 브랜치로 폴딩; dynamic 문맥은 미push(중복 없음); parity·회귀 0.
- I1(bounded)/I4(let-insertion)와 정합(중복 방지 원칙 공유).
- `pnix_runtime`/stage7/4-lane 무변경, `--check`/`--gate` 회귀 0.

## 남은 것 (P2, 선택)
- apply-through-if(정적 함수 문맥): `f (if c then a else b)`→`if c then (f a) else (f b)` — spec-point
  중복 위험 있어 신중 필요.
- let-bound if 피연산자(`let r = if..; in r op R`)까지 — 현재는 구문적 `(if..) op R`만.
- 전면 CPS specializer(Bondorf)로 deforestation까지 — 대규모, 실익 우리 워크로드 미검증(보류).

## Forbidden (지킴)
- dynamic 피연산자 push(중복 유발) 금지. `pnix_runtime`/stage7/4-lane 수정. 정본 평가기 대체.
