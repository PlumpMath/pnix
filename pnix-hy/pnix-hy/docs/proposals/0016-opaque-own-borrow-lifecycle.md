# 0016 — opaque-ref own/borrow 수명 규율 (0013 I2 승격)

- 상태: **SHIPPED 2026-07-02** (승격 수락 동일일). Additive only, `pnix_hy/interop.py` 한정.
- 근거: WebAssembly Component Model Canonical ABI(own/borrow 비트 + num_lends, drop-trap, call-scoped
  borrow) — 1차 출처 3-0 검증.
- Scope: proposal 0007의 lane-local lifecycle(카운터) 확장. **공유 ref shape(`__pnix_opaque__` 등)
  무변경**(SCOPE_LOCK §6 — shape을 바꾸면 양-레인+drift-guard 필요하므로 이번엔 메타데이터만
  lane-local 레지스트리에 둔다).

## 목표

현재 카운터(total/released)는 세지만 **막지 못한다**. own/borrow 규율로 위험한 해제를 오류로:
- own 핸들: 소유자만 release 가능; **미반환 대여(num_lends>0) 상태의 release는 InteropError**.
- borrow 핸들: `lend_opaque(ref)`로 발급, **call-scoped** — `with` 컨텍스트 종료 시 자동 반납;
  반납 없이 스코프 이탈 시 InteropError.

## 구현 (재사용만)

- `_OPAQUE_META`(lane-local dict)에 `{owned: bool, num_lends: int}` 추가 (ref shape 무변경).
- 신규 `interop.lend_opaque(ref)` → contextmanager: 진입 시 `num_lends += 1`, 이탈 시 `-= 1`.
- `release_opaque(ref)`: `num_lends > 0`이면 `InteropError('release while lent (num_lends=N)')`
  raise(D1 규약 — proposal 0006 재사용).
- `opaque_lifecycle()`/`opaque_lifecycle_report()`에 `lends_active`/`lend_violations` 카운트와
  own/borrow 검증 케이스(정상 대여-반납-해제 / 대여 중 해제 오류 / 이중 반납 오류) 추가.

## 수용 기준 / 시험

- 정상 시퀀스: make→lend(with)→해제 OK. 위반 시퀀스: lend 안에서 release → InteropError.
- `opaque_lifecycle_report()["ready"] is True` + **멱등**(연속 30회 동일 — 기존 멱등 회귀 유지 필수).
- 기존 opaque API(0002/0006/0007) 동작 불변, `--check` all_ready, `--gate` PASS.
