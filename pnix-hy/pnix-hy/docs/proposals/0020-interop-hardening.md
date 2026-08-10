# 0020 — interop 하드닝 웨이브 (0013 I1+I3+I4+I5+I6 승격)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현, "위 전부 승격해서 구현"). Additive, `pnix_hy/interop.py` 한정.
- 근거(전부 1차 출처 3-0 검증): SES(기본권한0+endow+**런타임 회수**), GraalVM(Context-수명 값),
  blame calculus(방향 판정), SES harden(전이 동결 표면), Trustworthy Proxies(기질-강제 불변식).
- 원칙: 공유 ref shape/witness envelope **무변경**(전부 lane-local 사이드카). 새 evaluator/gate 없음.

## 딜리버러블
1. **I1 회수 가능 capability**: `grant_capability(effects)` → 핸들(`attenuate/suspend/resume/revoke`).
   `check_capability`/`call_host` 계열이 granted 안의 핸들을 인식(회수·정지 시 즉시 거부).
2. **I3 context-수명 opaque**: `interop_context()` — 컨텍스트 안에서 만든 opaque-ref는 종료 시 일괄
   해제, 이후 접근은 `InteropError('context closed')`.
3. **I4 blame 방향**: 경계 실패의 `InteropError.blame ∈ {'host','pnix'}` — host 인자/호출 실패는
   host, pnix 평가 실패는 pnix(스키마 필드 아님 — 예외 속성 + error dict).
4. **I5 harden 표면-witness**: `harden_opaque(ref)` — 허용 메서드 표면을 해시 witness로 동결,
   호출마다 재검증(변조 시 `surface-tampered` 거부).
5. **I6 기질-강제 불변식**: `declare_opaque_invariants(ref, frozen_attrs)` — 단일 호출 진입점이
   호출 전 동결 속성 불변을 검증(래퍼 신뢰 불요).

## 수용: 신규 `interop_hardening_report` 등록(+1) — 회수/정지/컨텍스트닫힘/blame 양방향/변조검출/
불변식위반 케이스 전부 판정. 기존 interop 리포트 6종 회귀 0.
