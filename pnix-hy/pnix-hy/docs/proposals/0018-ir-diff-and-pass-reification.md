# 0018 — pnix IR 구조 diff + 패스 파이프라인 물화 (0013 G1+P3 승격)

- 상태: **SHIPPED 2026-07-02** (승격 수락 동일일). Additive only, `pnix_hy/ir.py` 중심.
- 근거: 감사 확인 진짜 gap G1(node-수준 IR diff 부재 — 해시 boolean 동등뿐, med 가치) +
  nanopass ICFP'13(define-language/extends 델타, Chez 상용 검증) 3-0 검증.
- Scope: pnix-hy 툴킷. IR 의미/스키마 무변경(관찰 도구만). sacred 무접촉.

## 목표

1. **`ir_diff(src_a, src_b)`**: 두 소스의 정규화 IR을 노드 수준으로 비교해
   `{equal, first_divergence_path, added, removed, changed}` 반환 — "해시가 다르다"에서
   "**어디가** 다르다"로. (drift 진단·specialize 검증·리뷰에 사용)
2. **패스 물화 `ir_pipeline()`**: `lower_to_ir`가 거치는 변환 단계를 nanopass 스타일
   **델타 시퀀스 데이터**로 노출 — 각 단계 `{pass, input_tags, output_tags, invariant}` +
   단계별 불변식 검사(태그 집합이 선언과 일치).

## 구현 (재사용만)

- `ir.ir_diff(a, b)`: `lower_to_ir` 두 번 → 구조 재귀 비교(경로는 `["let", "bindings", 0, ...]`
  형태). 리스트는 인덱스 정렬, dict 키는 정렬 순회로 **결정적** diff.
- `ir.ir_pipeline()`: 현재 lower 단계들을 하드코딩 선언 리스트로 물화(코드에서 파생 가능하면 파생) +
  각 단계 후 IR의 태그 집합이 선언된 `output_tags` ⊆ 관계인지 검사.
- 신규 `ir.ir_diff_report()`: (a) 동일 소스 diff → equal, (b) 한 리터럴만 바꾼 쌍 →
  `first_divergence_path`가 그 위치를 정확 지목, (c) 파이프라인 불변식 전 단계 통과.
- `cli._toolkit_reports()`에 `"ir_diff"` 등록 → `--check` **+1**. CLI `--ir-diff 'A ;; B'`는
  선택(하면 `_split_capability_spec` 수정본 재사용).

## 수용 기준 / 시험

- `ir_diff('let a=1; in a+2','let a=1; in a+3')`의 divergence 경로가 리터럴 위치를 지목.
- `ir_diff(x, x)["equal"] is True` (공백/포맷 차이 무시 — 정규화 IR 기준).
- `ir_diff_report()["ready"] is True`; `--check` all_ready(+1), `--gate` PASS.
