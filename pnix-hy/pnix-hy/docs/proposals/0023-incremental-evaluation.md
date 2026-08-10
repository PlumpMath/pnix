# 0023 — 증분 평가: 정의-단위 내용주소 + realisation 조기중단 (0013 R1+R3+G3 승격)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, 신규 `pnix_hy/incremental.py`.
- 근거: Unison(정의별 해시, 의존성은 해시로 치환, 이름=메타데이터), Nix CA(resolved derivation
  조기중단 + Realisation 매핑) — 3-0 검증. G3(cache-key 의존성 필드)를 함께 해소.

## 딜리버러블
1. **R1**: `definition_hashes(source)` — top-level let의 각 바인딩을 **의존성-해시-치환** 후 내용
   해시(위상 순회; 순환은 결합 해시 폴백). 이름 변경(α)은 해시 불변(이름=메타데이터).
2. **R1**: `incremental_eval(source)` — 순수 정의별 값 캐시(定義 해시 키). 한 바인딩만 바뀌면
   그 정의(+의존자)만 재계산. 미지원 형태는 `cached_eval` 폴백(건전성 우선).
3. **R3**: `realisation_record(source)` — `ir_hash → value_hash` **Realisation 스토어** + witness;
   resolved-hash 일치 시 평가 스킵(조기중단), hit/miss 카운터.

## 수용: 신규 `incremental_eval_report` 등록(+1) — 동일 소스 전량 히트 / 한 바인딩 변경 시 부분
재계산 / α-rename 히트 / realisation 조기중단 / 값 정확성(전체 eval과 동등) 판정.
