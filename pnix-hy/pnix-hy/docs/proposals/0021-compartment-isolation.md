# 0021 — compartment식 게스트 격리 (0013 I8 승격)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, 신규 `pnix_hy/compartment.py`.
- 근거: SES Compartment(구획별 globalThis + 훅 로더, 동결 intrinsic 공유) — 3-0 검증.

## 딜리버러블
`Compartment` — 구획별 **자기 env(바인딩 지속) + 자기 모듈 테이블**, builtins(순수 intrinsic)는
공유. `comp.eval(src)`(env 누적), `comp.bind(name, src)`, `comp.register_module(name, src)`(lazy
attrset로 env에 노출). 구획 간 바인딩·모듈 완전 격리.

## 수용: 신규 `compartment_report` 등록(+1) — A구획 바인딩이 B에 안 보임 / 모듈 격리 / 구획 내
지속 / builtins 공유 / granted 전파 판정. REPL(`repl.py`) 무변경(다음 단계에서 채택 가능).
