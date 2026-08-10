# 0022 — phase 산술 + 컴파일-실행 분리 게이트 (0013 P2+P4 승격)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, 신규 `pnix_hy/phase.py`.
- 근거: Flatt(phase ±정수 산술 합성·상쇄; macromod 빈-store 분리 증명) — 3-0 검증.

## 딜리버러블
1. **P2**: 투영 단계의 정수 phase 대수 — `PHASE_SHIFTS`(quote/for-syntax +1, unquote/for-template
   −1, eval 0), `phase_of(ops)` 합성, 상쇄·결합 법칙 검사 + 툴킷 표면(quasiquote depth 등) 매핑표.
2. **P4**: 관측 무관성 — lowering(`lower_to_ir`) N회가 런타임 상태(캐시/opaque/counter)를 **일절
   변경하지 않음**(빈-store 분리) + `eval(src) == eval_ir(lower(src))` 코퍼스 동등.

## 수용: 신규 `phase_separation_report` 등록(+1).
