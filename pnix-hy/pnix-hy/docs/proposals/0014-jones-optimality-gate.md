# 0014 — Jones-optimality 수용 게이트 (0013 T1 승격)

- 상태: **SHIPPED 2026-07-02** (승격 수락 동일일, 0013 우선순위 2 일괄 승격). Additive only.
- Scope: pnix-hy 툴킷 리포트 1개 추가. sacred 무접촉, 새 evaluator/mirror/gate 없음.
- 근거: Amin & Rompf POPL'18 (Pink, Prop 4.4/4.6 Multi-Level Jones Optimality) — 1차 출처 3-0 검증.

## 목표

기존 B==C 고정점 증명을 **모든 프로그램으로 정량화**하는 검사 가능 성질:
`compile(source(p))`의 정본이 `canonical(p)`와 **내용해시 동등**임을 코퍼스 전체에 게이트로 강제.

## 구현 (재사용만)

- 신규 `pnix_mirror.jones_optimality_report()`:
  1. 코퍼스 = `pnix_runtime`의 self-test 케이스 소스(이미 존재하는 케이스 목록 재사용; 최소 545 4-lane 코퍼스와 동일 소스 셋).
  2. 각 p에 대해 `canonical_a = ir.ir_of(p)["ir_sha256"]` (정본).
  3. `emitted = pnix_runtime`의 canonical emit(기존 `emitted_source`/`emit` 경로 재사용)으로 p를 소스로 재방출 → 재파싱 → `canonical_b = ir.ir_of(emitted)["ir_sha256"]`.
  4. `optimal = (canonical_a == canonical_b)`; 카운트 집계, 전부 통과 시 `ready: True`.
  5. (2단계, 선택) n-단 자기해석 붕괴: stage7 커널로 p를 평가한 값 == host 평가 값 해시 동등(이미 4-lane이 함) + emit∘parse 반복 2회 고정점(`emit(parse(emit(parse(p)))) == emit(parse(p))`).
- `cli._toolkit_reports()`에 `"jones_optimality"` 등록 → `--check` **57 → 58**.

## 수용 기준 / 시험

- `jones_optimality_report()["ready"] is True`, 코퍼스 전수(>=545 소스) 해시 동등.
- 고의 회귀 시험: emit를 임시로 변형하면(공백 아닌 구조 변화) 리포트가 FAIL로 떨어짐을 확인 후 원복.
- `--check` 58/58, `--gate` PASS(회귀 0).
