# 0017 — hygiene/symbol-capture self-check 리포트 (0013 P1+G4 승격)

- 상태: **SHIPPED 2026-07-02** (승격 수락 동일일). Additive only.
- 근거: Flatt POPL'16 sets-of-scopes(위생 해석 = 집합 연산, 3-0 검증) + 감사 확인 진짜 gap G4
  (pnix-hy 툴킷에 hygiene 리포트 부재 — hy-meta 테스트 레인에만 증거 존재).
- Scope: pnix-hy 툴킷 리포트 1개. pnix 비동형 원칙 무접촉(검사 대상은 **Hy-쪽 매크로 브리지**).

## 목표

`hy_macro_over_pnix`/`hy_quasiquote_over_pnix`(proposal 0003) 경로에서 **의도치 않은 심볼 포획이
없음**을 실행 가능한 리포트로 증명. 관점: 매크로가 도입한 바인딩이 사용부(pnix-투영 폼)의 자유
심볼을 포획하면 FAIL.

## 구현 (재사용만)

- 신규 `pnix_mirror.hygiene_report()`:
  1. **포획 시도 케이스**: 사용부 폼이 자유 변수 `x`를 갖고, 매크로가 `x`를 바인딩하는 정의
     (`(defmacro m [form] \`(let [x 999] ~form))`)를 `hy_macro_over_pnix`로 적용 →
     확장 결과에서 사용부 `x`의 해석이 999가 **되면 포획 = 위생 위반 기록**(Hy 1.3.0 gensym 유무
     실측 — 결과가 어느 쪽이든 리포트는 사실을 기록하고, 위반 케이스는 `capture_detected` 필드로 명시).
  2. **gensym 케이스**: `hy.gensym` 기반 매크로는 포획 0임을 확인.
  3. **scope-set 근사 검사**: 확장 전/후 폼의 심볼 집합 diff(도입/소거 심볼 목록)를 데이터로 물화
     — 도입 심볼이 사용부 자유 심볼과 충돌하면 `collisions`에 기록.
  4. `ready` = 케이스 전부 기대 판정과 일치(gensym 케이스 포획 0 + 충돌 검출기가 심은 충돌을 잡음).
- `hy_mirror.hy_eval_form`/`hy_macroexpand_projection` 재사용(새 Hy 기계 금지).
- `cli._toolkit_reports()`에 `"hygiene"` 등록 → `--check` **+1**.

## 수용 기준 / 시험

- 심은 포획 케이스를 검출(`capture_detected` 또는 `collisions` 비어있지 않음), gensym 케이스 통과.
- `hygiene_report()["ready"] is True`; Hy 부재 환경에선 `available:False`로 우아하게 물러남.
- `--check` all_ready(+1), `--gate` PASS.
