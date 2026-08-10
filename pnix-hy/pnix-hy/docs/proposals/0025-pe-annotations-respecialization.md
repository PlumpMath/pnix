# 0025 — PE 어노테이션 + 의미변경 재특화 (0013 T4+T7 승격)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, `pnix_hy/pnix_mirror.py` 한정.
- 근거: Truffle/Graal PLDI'17(무유도 전체 PE는 불가 — PEFinal/Assumption/PEBoundary 소량 프리미티브),
  Purple(수정된 의미 아래 재컴파일) — 3-0 검증.

## 딜리버러블
1. **T4**: `specialize_pnix(source, dynamic_vars, *, assumptions=None, boundaries=())` —
   `assumptions`(이름→가정값: static 취급 + 잔여 레코드에 기록), `boundaries`(강제 dynamic =
   PEBoundary). 기본 인자 시 기존 동작 byte-동일.
2. **T4**: `assumptions_valid(record, env)` — deopt 판정(가정 위반 검출).
3. **T7**: `respecialize_if_drifted(source, dynamic_vars, env, record)` — 가정 위반 시 캐시 무효화
   + 재특화 + 재특화 witness. 위반 없으면 기존 레코드 재사용.

## 수용: 신규 `pe_annotations_report` 등록(+1) — 가정부 특화 값 정확 / boundary 강제 dynamic /
deopt 검출 / 재특화 카운터·witness / 무가정 경로 회귀 0(A4/A5/A15 수정 유지) 판정.
