# 0015 — 경계 수치 무손실 술어 fitsIn* (0013 I7 승격)

- 상태: **SHIPPED 2026-07-02** (승격 수락 동일일). Additive only, `pnix_hy/interop.py` 한정.
- 근거: GraalVM polyglot Value API(fitsInByte/Int/Long/BigInteger + as* 실패 규약) — 1차 출처 3-0 검증.
- Scope: 기존 loss 마킹(A-계열, proposal 0001)의 자연 확장. ABI envelope 무변경.

## 목표

수치가 경계를 넘을 때 손실을 **변환 전에 술어로 판정**해 명시 기록. 현재 big-int↔float 등
정밀도 손실이 암묵적으로 통과한다.

## 구현 (재사용만)

- 신규 `interop.numeric_fits(value, kind)` — kind ∈ {"int","float","json-number"}:
  - int→float: `float(v)` 왕복(`int(float(v)) == v`)으로 53-bit 정밀도 판정.
  - float→int: `v.is_integer()`.
  - json-number: |v| ≤ 2^53−1 (JSON 안전 정수 범위).
- `from_host`/`to_host`의 수치 경로에서 술어 실패 시 `loss_status='lossy'` + `loss_reason='numeric-precision'`
  기록(값 자체는 기존 동작 유지 — 마킹만 추가; 값 변경 금지).
- `roundtrip_host_value` probe 셋에 수치 경계 케이스 추가: `2**53+1`, `10**30`, `0.1`, `float('inf')`,
  `float('nan')`(NaN은 equal=False가 정상 — 별도 `nan` 플래그).
- 리포트: 신규 등록 없이 기존 `interop_roundtrip`(=`roundtrip_report`)에 `numeric` 섹션 추가
  (`--check` 카운트 불변).

## 수용 기준 / 시험

- `to_host`/`from_host(2**53+1 → float 경유)` 손실이 lossy로 표기; 기존 케이스 마킹 불변(회귀 0).
- `roundtrip_report()["ready"] is True` + numeric 섹션 전 케이스 판정 일치.
- `--check` all_ready, `--gate` PASS.
